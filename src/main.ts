import { createApp } from 'vue'
import { createPinia } from 'pinia'

// Import Tui.Image-Editor CSS globally
import 'tui-image-editor/dist/tui-image-editor.css'
import './assets/tui-image-editor-override.css'


import App from './App.vue'
import router from './router'
import './assets/main.css'



const app = createApp(App)

app.use(createPinia())
app.use(router)

app.mount('#app')
