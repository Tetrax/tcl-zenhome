import { mount } from "svelte";
import "./app.css";
import App from "./TestReseau.svelte"; // écran de test 3a ; l'écran normal : ./App.svelte

export default mount(App, { target: document.getElementById("app")! });
