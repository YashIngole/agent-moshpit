import { mount } from 'svelte'
import App from './App.svelte'
import './styles/base.css'

mount(App, { target: document.getElementById('app')! })
