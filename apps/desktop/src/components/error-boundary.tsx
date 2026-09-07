import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  render() {
    if (this.state.error) {
      return this.props.fallback ?? (
        <div
          className="flex flex-col items-center justify-center py-16 gap-3"
          style={{ color: "var(--color-bad)" }}
        >
          <div className="text-[36px]">⚠</div>
          <div className="text-[14px] font-medium">Something went wrong</div>
          <div className="text-[12px] max-w-md text-center" style={{ color: "var(--color-text-sec)" }}>
            {this.state.error.message}
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
