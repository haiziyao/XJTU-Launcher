import { createApp } from 'vue';
import IconCompare from './IconCompare.vue';
import '../styles/style.css';
import '../styles/ui.css';

// Dev-only comparison sheet for the settings surface; not part of the app.
createApp(IconCompare).mount('#app');
