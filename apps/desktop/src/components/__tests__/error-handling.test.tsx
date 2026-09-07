import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, fireEvent } from "@testing-library/react";
import { ErrorBoundary } from "@/components/error-boundary";
import { ToastProvider, useToast } from "@/components/toast";

const Throws = ({ message }: { message: string }) => {
  throw new Error(message);
};

describe("ErrorBoundary", () => {
  it("renders children when no error", () => {
    render(<ErrorBoundary><p>all good</p></ErrorBoundary>);
    expect(screen.getByText("all good")).toBeInTheDocument();
  });

  it("catches render errors and shows fallback", () => {
    const onError = vi.fn();
    const original = window.onerror;
    window.onerror = onError;

    render(
      <ErrorBoundary>
        <Throws message="boom" />
      </ErrorBoundary>
    );

    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    expect(screen.getByText("boom")).toBeInTheDocument();

    window.onerror = original;
  });

  it("renders custom fallback", () => {
    const onError = vi.fn();
    const original = window.onerror;
    window.onerror = onError;

    render(
      <ErrorBoundary fallback={<div data-testid="custom">Custom error</div>}>
        <Throws message="boom" />
      </ErrorBoundary>
    );

    expect(screen.getByTestId("custom")).toBeInTheDocument();
    expect(screen.getByText("Custom error")).toBeInTheDocument();

    window.onerror = original;
  });
});

describe("Toast system", () => {
  function ToastTester() {
    const { toast, toasts, dismiss } = useToast();
    return (
      <div>
        <button onClick={() => toast("hello")}>success</button>
        <button onClick={() => toast("error msg", "error")}>error</button>
        {toasts.map(t => (
          <div key={t.id} onClick={() => dismiss(t.id)} data-testid="toast">
            {t.message}
          </div>
        ))}
      </div>
    );
  }

  beforeEach(() => {
    vi.useRealTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows a success toast", async () => {
    render(
      <ToastProvider>
        <ToastTester />
      </ToastProvider>
    );

    fireEvent.click(screen.getByText("success"));
    expect(screen.getByText("hello")).toBeInTheDocument();
  });

  it("shows an error toast", async () => {
    render(
      <ToastProvider>
        <ToastTester />
      </ToastProvider>
    );

    fireEvent.click(screen.getByText("error"));
    expect(screen.getByText("error msg")).toBeInTheDocument();
  });

  it("dismisses toast on click", () => {
    render(
      <ToastProvider>
        <ToastTester />
      </ToastProvider>
    );

    fireEvent.click(screen.getByText("success"));
    expect(screen.getAllByTestId("toast")).toHaveLength(1);

    fireEvent.click(screen.getByTestId("toast"));
    expect(screen.queryByTestId("toast")).not.toBeInTheDocument();
  });

  it("supports multiple toasts", () => {
    render(
      <ToastProvider>
        <ToastTester />
      </ToastProvider>
    );

    fireEvent.click(screen.getByText("success"));
    fireEvent.click(screen.getByText("error"));
    expect(screen.getAllByTestId("toast")).toHaveLength(2);
  });

  it("auto-dismisses after 4 seconds", () => {
    vi.useFakeTimers();
    render(
      <ToastProvider>
        <ToastTester />
      </ToastProvider>
    );

    fireEvent.click(screen.getByText("success"));
    expect(screen.queryByTestId("toast")).toBeInTheDocument();

    act(() => { vi.advanceTimersByTime(4000); });
    expect(screen.queryByTestId("toast")).not.toBeInTheDocument();
    vi.useRealTimers();
  });
});
