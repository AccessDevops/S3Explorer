import { createApp } from 'vue'
import { createPinia } from 'pinia'

// Import Tui.Image-Editor CSS globally
import 'tui-image-editor/dist/tui-image-editor.css'
import './assets/tui-image-editor-override.css'


import App from './App.vue'
import router from './router'
import './assets/main.css'

// Directives
import { vTooltip } from './directives/tooltip'

const app = createApp(App)

app.use(createPinia())
app.use(router)

// Register global directives
app.directive('tooltip', vTooltip)

app.mount('#app')
