export type Item = {
  flavor: "tsubuan" | "custard" | "kurikinton",
  quantity: number,
}

export type OrdersDisplayResponse = {
  ready: { id: number }[],
  cooking: { id: number }[],
  waiting: { id: number }[],
}

export type OrdersIdResponse = {
  id: number,
  items: Item[],
  status: "waiting" | "cooking" | "ready" | "completed" | "cancelled",
  orderedAt: string,
  estimatedWaitMinutes: number,
}

export type WaitTimesResponse = {
  waitTimes: {
    tsubuan: number | null,
    custard: number | null,
    kurikinton: number | null,
  },
}
