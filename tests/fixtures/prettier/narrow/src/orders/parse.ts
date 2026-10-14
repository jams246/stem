import {Order, LineItem} from './types'

export function parseOrder(
    text: string,
    strict: boolean
): Order | null {
    const lines = text
        .split('\n')
        .filter(l => l.trim() !== '')
    if (lines.length === 0) return null
    const header = lines[0].split(',')
    const order: Order = {
        id: header[0],
        customer: header[1],
        items: [],
        express: header[2] === 'yes'
    }
    for (const line of lines.slice(1)) {
        const [sku, qty, price] =
            line.split(',')
        if (!sku) {
            if (strict)
                throw new Error(
                    'missing sku'
                )
            continue
        }
        const item: LineItem = {
            sku,
            quantity: Number(qty),
            price: Number(price)
        }
        if (
            item.quantity > 0 &&
            item.price >= 0
        ) {
            order.items.push(item)
        } else if (strict) {
            throw new Error(
                `bad line: ${line}`
            )
        }
    }
    return order
}

export const isEmpty = (order: Order) =>
    order.items.length === 0
