(module
  (import "env" "host_log" (func $log (param i32 i32)))
  (memory (export "memory") 2)
  (global $next (mut i32) (i32.const 4096))
  (data (i32.const 1024) "{\"summary\":\"Example plugin: review authentication and session handling for the engagement.\",\"testCases\":[{\"title\":\"Review authentication and session handling\",\"category\":\"Authentication\",\"approach\":\"Inspect login, logout, password reset, session rotation, and cookie attributes.\",\"expectedResult\":\"Sessions rotate and unauthorized access is blocked.\"}],\"findings\":[],\"notes\":[\"This is a synthetic example plugin.\"]}")
  (func (export "alloc") (param $len i32) (result i32)
    (local $p i32)
    global.get $next
    local.set $p
    global.get $next
    local.get $len
    i32.add
    global.set $next
    local.get $p)
  (func (export "run") (param $ptr i32) (param $len i32) (result i64)
    i32.const 1024
    i32.const 415
    call $log
    i64.const 4398046511519))
