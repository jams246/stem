import type { Order } from "../orders/types";

const RATES = { standard: 5, express: 20, "free-over": 100 };

export function subtotal(order: Order): number {
  return order.items.reduce((sum, item) => sum + item.price * item.quantity, 0);
}

export function applyDiscounts(order: Order, code?: string): number {
  let total = subtotal(order);
  if (code) {
    if (code === "HALF") {
      total = total / 2;
    } else if (code === "TEN") {
      if (total > 10) {
        total = total - 10;
      } else {
        for (const item of order.items) {
          if (item.price > 5) {
            total = total - 1;
          }
        }
      }
    }
  }
  return total < 0 ? 0 : total;
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
