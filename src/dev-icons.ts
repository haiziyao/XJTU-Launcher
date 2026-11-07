import { createApp } from 'vue';
import IconCompare from './components/dev/IconCompare.vue';
import './style.css';
import './ui.css';

// Dev-only comparison sheet for the settings surface; not part of the app.
createApp(IconCompare).mount('#app');
