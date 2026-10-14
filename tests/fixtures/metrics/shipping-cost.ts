function shippingCost(order: Order) {
  if (!order.items.length) return 0;
  let cost = 0;
  for (const item of order.items) {
    if (item.oversized) {
      cost += 20;
    } else {
      cost += 5;
    }
  }
  return order.express && !order.member ? cost * 2 : cost;
}
