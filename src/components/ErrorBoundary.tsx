import { Component, type ErrorInfo, type ReactNode } from "react";
import { AlertTriangle, RotateCcw } from "lucide-react";

type Props = { children: ReactNode };
type State = { error: Error | null };

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("TestTracker render error", error, info);
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="flex min-h-full items-center justify-center bg-slate-950 p-6">
        <div className="w-full max-w-lg rounded-xl border border-red-500/30 bg-slate-900 p-6">
          <div className="flex items-center gap-3">
            <AlertTriangle className="h-5 w-5 text-red-400" />
            <h1 className="text-sm font-semibold text-white">The interface hit an error</h1>
          </div>
          <p className="mt-3 text-xs leading-relaxed text-slate-400">
            Your encrypted data is safe. Reload the interface to continue. If this keeps happening,
            close and reopen TestTracker.
          </p>
          <pre className="mt-3 max-h-40 overflow-auto rounded-lg border border-slate-800 bg-slate-950 p-3 text-[11px] text-red-300">
            {this.state.error.message}
          </pre>
          <div className="mt-4 flex gap-2">
            <button
              type="button"
              className="inline-flex h-10 items-center gap-2 rounded-lg bg-emerald-500 px-4 text-sm font-medium text-slate-950 hover:bg-emerald-400"
              onClick={() => window.location.reload()}
            >
              <RotateCcw className="h-4 w-4" /> Reload interface
            </button>
            <button
              type="button"
              className="inline-flex h-10 items-center rounded-lg border border-slate-700 bg-slate-900 px-4 text-sm text-slate-200 hover:bg-slate-800"
              onClick={() => this.setState({ error: null })}
            >
              Try again
            </button>
          </div>
        </div>
      </div>
    );
  }
}
