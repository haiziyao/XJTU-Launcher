import { createApp } from 'vue';
import App from './App.vue';
import PetWindow from './PetWindow.vue';
import './style.css';

// The floating pet is a separate window of the same bundle; it only mounts the
// pet component and never receives the main window's IPC surface.
const petWindow = new URLSearchParams(window.location.search).get('window') === 'pet';
createApp(petWindow ? PetWindow : App).mount('#app');
