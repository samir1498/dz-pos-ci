// A CSS custom property is a legal thing to put in a `style` attribute and
// React passes it straight through, but `React.CSSProperties` lists only the
// named properties, so `style={{ "--sidebar-width": "15rem" }}` is a type
// error without help. shadcn's own components answer that with
// `as React.CSSProperties` at each site; `as` is banned here
// (context/processes/coding-rules), and a cast would switch off the checking
// on the whole object rather than on the one key that needs it.
//
// So the type is widened once, for every file, to say what the DOM already
// accepts: a key starting with two dashes carries a string or a number.
// Everything else in the object keeps its usual type, which is what a cast
// would have thrown away.
//
// The bare `import "react"` is what makes this file a module: in a global
// script file `declare module "react"` would declare a new ambient module
// and shadow React's own types instead of merging with them.

import "react";

declare module "react" {
  interface CSSProperties {
    [key: `--${string}`]: string | number | undefined;
  }
}
