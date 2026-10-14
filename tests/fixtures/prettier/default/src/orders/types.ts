export interface LineItem {
  sku: string;
  quantity: number;
  price: number;
}
export interface Order {
  id: string;
  customer: string;
  items: LineItem[];
  express: boolean;
}
