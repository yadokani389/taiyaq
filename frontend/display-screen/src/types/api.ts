export interface OrderDisplay {
  id: number
}

export interface DisplayResponse {
  ready: OrderDisplay[]
  cooking: OrderDisplay[]
  waiting: OrderDisplay[]
}
