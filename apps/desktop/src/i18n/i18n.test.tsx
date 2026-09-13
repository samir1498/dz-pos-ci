import { afterEach, expect, test } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider, useTranslation } from "./index";
import { LanguageSwitcher } from "./LanguageSwitcher";
import ar from "./ar.json";

function Probe() {
  const { t, dir } = useTranslation();
  return <span data-testid="p" dir={dir}>{t("app_name")}</span>;
}

/** Renders a key that actually differs by language; app_name does not
 * (it is "Dinar" in all three dictionaries on purpose). */
function TitleProbe() {
  const { t } = useTranslation();
  return <span data-testid="title">{t("products_title")}</span>;
}

afterEach(() => {
  localStorage.clear();
});

test("arabic is rtl, others ltr", () => {
  render(<I18nProvider lang="ar"><Probe /></I18nProvider>);
  expect(screen.getByTestId("p")).toHaveAttribute("dir", "rtl");
});

test("french is the default and ltr", () => {
  render(<I18nProvider><Probe /></I18nProvider>);
  expect(screen.getByTestId("p")).toHaveAttribute("dir", "ltr");
  expect(screen.getByTestId("p")).toHaveTextContent("Dinar");
});

test("the switcher changes lang, dir and the document attributes", async () => {
  const user = userEvent.setup();
  render(
    <I18nProvider>
      <LanguageSwitcher />
      <Probe />
    </I18nProvider>,
  );
  expect(screen.getByTestId("p")).toHaveAttribute("dir", "ltr");
  expect(document.documentElement).toHaveAttribute("lang", "fr");

  const arButton = screen.getByRole("button", { name: "العربية" });
  expect(arButton).toHaveAttribute("aria-pressed", "false");
  await user.click(arButton);

  expect(screen.getByTestId("p")).toHaveAttribute("dir", "rtl");
  expect(document.documentElement).toHaveAttribute("dir", "rtl");
  expect(document.documentElement).toHaveAttribute("lang", "ar");
  expect(arButton).toHaveAttribute("aria-pressed", "true");
});

test("a screen renders the Arabic label of a known key under ar", () => {
  render(<I18nProvider lang="ar"><TitleProbe /></I18nProvider>);
  expect(screen.getByTestId("title")).toHaveTextContent(ar.products_title);
});

test("the persisted choice is read at start", () => {
  localStorage.setItem("dzpos-lang", "ar");
  render(<I18nProvider><Probe /></I18nProvider>);
  expect(screen.getByTestId("p")).toHaveAttribute("dir", "rtl");
});
