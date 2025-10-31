import { createMemoryHistory, createRouter } from 'vue-router'

import HomeView from './views/Home.vue'
import WaitTimeView from './views/WaitTime.vue'

const routes = [
  { path: '/', component: HomeView },
  { path: '/wait-time', component: WaitTimeView },
]

export const router = createRouter({
  history: createMemoryHistory(),
  routes,
})
