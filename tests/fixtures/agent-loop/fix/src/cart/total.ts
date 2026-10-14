import type { Order } from "../orders/types";

const RATES = { standard: 5, express: 20, "free-over": 100 };

export function subtotal(order: Order): number {
  return order.items.reduce((sum, item) => sum + item.price * item.quantity, 0);
}

const DISCOUNTS: Record<string, (total: number) => number> = {
  HALF: (total) => total / 2,
  TEN: (total) => total - 10,
};

export function applyDiscounts(order: Order, code?: string): number {
  const discount = code ? DISCOUNTS[code] : undefined;
  const total = discount ? discount(subtotal(order)) : subtotal(order);
  return Math.max(0, total);
}

export class Checkout {
  constructor(private readonly order: Order, private readonly currency = "EUR") {}

  get shipping(): number {
    const base = this.order.express ? RATES.express : RATES.standard;
    return subtotal(this.order) > RATES["free-over"] ? 0 : base;
  }

  summary(): string {
    const lines = this.order.items.map(item => `${item.sku} x${item.quantity}`);
    return [...lines, `shipping ${this.shipping} ${this.currency}`].join("\n");
  }
}
