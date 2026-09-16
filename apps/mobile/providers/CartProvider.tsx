// The basket, held above the auth gate on purpose.
//
// A cashier is mid-sale, three items in, and the fifteen-minute idle rule
// kills the session. The gate redirects to the sign-in route, which unmounts
// everything below it — and if the basket lived in the till screen's own
// state, those three items would be gone by the time the PIN is typed. In a
// shop that means the customer's things get scanned again with the customer
// standing there.
//
// So the cart lives here, outside `(signed-in)`, and the till reads it. The
// same reasoning is why the retry queue is on disk rather than in a screen.

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

import type { CartLine, Product } from "../lib/basket";

type CartState = {
  lines: readonly CartLine[];
  add: (product: Product) => void;
  remove: (productId: number) => void;
  clear: () => void;
};

const CartContext = createContext<CartState | null>(null);

export function useCart(): CartState {
  const value = useContext(CartContext);
  if (value === null) throw new Error("useCart outside CartProvider");
  return value;
}

export function CartProvider({ children }: { children: ReactNode }) {
  const [lines, setLines] = useState<readonly CartLine[]>([]);

  const add = useCallback((product: Product) => {
    setLines((current) => {
      const found = current.find((line) => line.product.id === product.id);
      if (found !== undefined) {
        return current.map((line) =>
          line.product.id === product.id ? { ...line, qty: line.qty + 1 } : line,
        );
      }
      return [...current, { product, qty: 1 }];
    });
  }, []);

  const remove = useCallback((productId: number) => {
    setLines((current) =>
      current
        .map((line) => (line.product.id === productId ? { ...line, qty: line.qty - 1 } : line))
        .filter((line) => line.qty > 0),
    );
  }, []);

  const clear = useCallback(() => setLines([]), []);

  const value = useMemo(() => ({ lines, add, remove, clear }), [lines, add, remove, clear]);
  return <CartContext.Provider value={value}>{children}</CartContext.Provider>;
}
