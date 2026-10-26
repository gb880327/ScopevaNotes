/**
 * Entry point for the 速记 panel window.
 *
 * Deliberately separate from `main.ts`: the panel must not pull in the note
 * editor, the preview pipeline or their dependencies (design D14). Only the
 * store, the panel components and the shared styles are imported here.
 */
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import QuickTabsApp from './windows/quick-tabs/QuickTabsApp.vue'
import './styles/app.css'

createApp(QuickTabsApp).use(createPinia()).mount('#quick-tabs')
