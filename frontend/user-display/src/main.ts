import { createApp } from 'vue'
import { router } from './router';
import App from './App.vue'
import PrimeVue from 'primevue/config';
import Aura from '@primeuix/themes/aura';
import 'primeicons/primeicons.css'
import "./assets/base.css"

createApp(App).use(router).use(PrimeVue, {
  theme: {
    preset: Aura,
    options: {
      darkModeSelector: 'none',
    }
  }
}).mount('#app')
