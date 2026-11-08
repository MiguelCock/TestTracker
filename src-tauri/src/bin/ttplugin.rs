use std::fs;
use std::io::Write;
use std::path::Path;

use testtracker_lib::plugins::package;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, secret_out] if command == "keygen" => {
            let (secret, public) = package::generate_keypair();
            write_private(Path::new(secret_out), secret.as_bytes())?;
            println!("{public}");
            println!("Secret key written to {secret_out}. Import the public key in TestTracker.");
            Ok(())
        }
        [command, plugin_dir, secret_file, output] if command == "sign" => {
            let secret = fs::read_to_string(secret_file)
                .map_err(|error| format!("unable to read secret key: {error}"))?
                .trim()
                .to_string();
            let key = package::signing_key_from_hex(&secret)
                .map_err(|error| format!("invalid secret key: {error}"))?;
            let public = hex::encode(key.verifying_key().to_bytes());
            let manifest = fs::read(Path::new(plugin_dir).join("manifest.json"))
                .map_err(|error| format!("unable to read manifest.json: {error}"))?;
            let wasm_path = Path::new(plugin_dir).join("plugin.wasm");
            let wasm = if wasm_path.is_file() {
                fs::read(&wasm_path)
                    .map_err(|error| format!("unable to read plugin.wasm: {error}"))?
            } else {
                let wat_path = Path::new(plugin_dir).join("plugin.wat");
                wat::parse_file(&wat_path)
                    .map_err(|error| format!("unable to compile plugin.wat: {error}"))?
            };
            let signature = package::sign_package(&manifest, &wasm, &key);
            let bytes = package::create_package(&manifest, &wasm, &signature)
                .map_err(|error| format!("unable to create package: {error}"))?;
            write_private(Path::new(output), &bytes)?;
            println!("Package: {output}");
            println!("Publisher public key: {public}");
            Ok(())
        }
        [command, package_path, public_key] if command == "verify" => {
            let bytes = fs::read(package_path)
                .map_err(|error| format!("unable to read package: {error}"))?;
            let parsed = package::parse_package(&bytes)
                .map_err(|error| format!("invalid package: {error}"))?;
            if !package::verify_signature(&parsed, public_key) {
                return Err("signature verification failed".into());
            }
            println!(
                "verified {} {} from {}",
                parsed.manifest.id, parsed.manifest.version, parsed.manifest.publisher_id
            );
            Ok(())
        }
        _ => {
            eprintln!(
                "usage:\n  ttplugin keygen <secret-out>\n  ttplugin sign <plugin-dir> <secret-key-file> <output.ttplugin>\n  ttplugin verify <package.ttplugin> <public-key-hex>"
            );
            std::process::exit(2);
        }
    }
}

fn write_private(path: &Path, contents: &[u8]) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("unable to write {}: {error}", path.display()))?;
    file.write_all(contents)
        .map_err(|error| format!("unable to write {}: {error}", path.display()))?;
    Ok(())
}
