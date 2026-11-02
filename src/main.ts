import { createApp } from 'vue';
import App from './App.vue';
import { installEditorThemes } from './lib/editor-theme';
import './style.css';
import './features/editor/editor-chrome.css';

installEditorThemes();
createApp(App).mount('#app');
