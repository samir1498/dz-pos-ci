import { expect, test } from "vitest";
import { render, screen } from "@testing-library/react";
import { I18nProvider, useTranslation } from "./index";

function Probe() {
  const { t, dir } = useTranslation();
  return <span data-testid="p" dir={dir}>{t("app_name")}</span>;
}

test("arabic is rtl, others ltr", () => {
  render(<I18nProvider lang="ar"><Probe /></I18nProvider>);
  expect(screen.getByTestId("p")).toHaveAttribute("dir", "rtl");
});

test("french is the default and ltr", () => {
  render(<I18nProvider><Probe /></I18nProvider>);
  expect(screen.getByTestId("p")).toHaveAttribute("dir", "ltr");
  expect(screen.getByTestId("p")).toHaveTextContent("dz-pos");
});
