import { Order } from '../orders/types';

type Props = { order: Order; onRemove: (sku: string) => void; compact?: boolean };

export function CartView({ order, onRemove, compact }: Props) {
  if (order.items.length === 0) {
    return <p className='empty'>Your cart is empty.</p>;
  }
  return (
    <ul className={compact ? 'cart compact' : 'cart'}>
      {order.items.map(item => (
        <li key={item.sku}>
          {item.sku} ({item.quantity})
          {!compact && <button onClick={() => onRemove(item.sku)}>Remove</button>}
        </li>
      ))}
    </ul>
  );
}
