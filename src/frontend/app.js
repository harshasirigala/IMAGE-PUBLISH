const API = 'http://127.0.0.1:8080';
let token = localStorage.getItem('token');
let username = localStorage.getItem('username');
let activeCam = null;
let ws = null;

// ── Init ──────────────────────────────────────────────────────────────────
window.onload = () => {
  if (token) showDashboard();
};

// ── Auth ──────────────────────────────────────────────────────────────────
async function login() {
  const u = document.getElementById('username').value;
  const p = document.getElementById('password').value;
  const res = await fetch(`${API}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: u, password: p })
  });
  const data = await res.json();
  if (!res.ok) {
    document.getElementById('login-error').textContent = data.error;
    return;
  }
  token = data.token;
  username = data.username;
  localStorage.setItem('token', token);
  localStorage.setItem('username', username);
  showDashboard();
}

async function register() {
  const u = document.getElementById('username').value;
  const p = document.getElementById('password').value;
  const res = await fetch(`${API}/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: u, password: p })
  });
  const data = await res.json();
  if (!res.ok) {
    document.getElementById('login-error').textContent = data.error;
    return;
  }
  token = data.token;
  username = data.username;
  localStorage.setItem('token', token);
  localStorage.setItem('username', username);
  showDashboard();
}

function logout() {
  localStorage.removeItem('token');
  localStorage.removeItem('username');
  token = null;
  if (ws) ws.close();
  document.getElementById('dashboard-page').classList.add('hidden');
  document.getElementById('login-page').classList.remove('hidden');
}

// ── Dashboard ─────────────────────────────────────────────────────────────
async function showDashboard() {
  document.getElementById('login-page').classList.add('hidden');
  document.getElementById('dashboard-page').classList.remove('hidden');
  document.getElementById('header-user').textContent = username;

  await loadCameras();
  await loadEvents();
  connectWebSocket();
}

// ── Cameras ───────────────────────────────────────────────────────────────
async function loadCameras() {
  const res = await fetch(`${API}/api/cameras`, {
    headers: { 'Authorization': `Bearer ${token}` }
  });
  const cameras = await res.json();

  const container = document.getElementById('camera-cards');
  const tabs = document.getElementById('cam-tabs');
  container.innerHTML = '';
  tabs.innerHTML = '';

  cameras.forEach((cam, i) => {
    // Card
    const card = document.createElement('div');
    card.className = 'cam-card';
    card.innerHTML = `
      <div class="cam-name">${cam.camera_id}</div>
      <div class="cam-status">
        <div class="dot green"></div>
        <span>${cam.status}</span>
      </div>
      <button class="capture-btn" onclick="sendCapture('${cam.camera_id}')">
        📸 Capture
      </button>
    `;
    container.appendChild(card);

    // Tab
    const tab = document.createElement('div');
    tab.className = 'tab' + (i === 0 ? ' active' : '');
    tab.textContent = cam.camera_id;
    tab.onclick = () => {
      document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      loadImages(cam.camera_id);
    };
    tabs.appendChild(tab);
  });

  // Load images for first camera
  if (cameras.length > 0) {
    activeCam = cameras[0].camera_id;
    loadImages(activeCam);
  }
}

// ── Capture Command ───────────────────────────────────────────────────────
async function sendCapture(cam) {
  const res = await fetch(`${API}/api/command/${cam}`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ command: 'capture' })
  });
  const data = await res.json();
  console.log('Command sent:', data);
}

// ── Events ────────────────────────────────────────────────────────────────
async function loadEvents() {
  const res = await fetch(`${API}/api/events`, {
    headers: { 'Authorization': `Bearer ${token}` }
  });
  const events = await res.json();

  const tbody = document.getElementById('events-table');
  tbody.innerHTML = '';
  events.forEach(ev => {
    const tr = document.createElement('tr');
    tr.innerHTML = `
      <td>${ev.device_id}</td>
      <td>${new Date(ev.timestamp).toLocaleString()}</td>
      <td><a href="${ev.image_url}" target="_blank">View Image</a></td>
      <td>${ev.bucket}</td>
    `;
    tbody.appendChild(tr);
  });
}

// ── Images ────────────────────────────────────────────────────────────────
async function loadImages(cam) {
  activeCam = cam;
  const res = await fetch(`${API}/api/images/${cam}`, {
    headers: { 'Authorization': `Bearer ${token}` }
  });
  const images = await res.json();

  const gallery = document.getElementById('image-gallery');
  gallery.innerHTML = '';
  images.forEach(img => {
    const el = document.createElement('img');
    el.src = img.url;
    el.alt = img.key;
    el.title = img.key;
    gallery.appendChild(el);
  });
}

// ── WebSocket ─────────────────────────────────────────────────────────────
function connectWebSocket() {
  ws = new WebSocket(`ws://127.0.0.1:8080/ws`);

  ws.onopen = () => console.log('WebSocket connected');

  ws.onmessage = (e) => {
    try {
      const data = JSON.parse(e.data);
      addLiveEvent(data);
      // Refresh table and gallery
      loadEvents();
      if (activeCam === data.device_id) loadImages(activeCam);
    } catch (_) {}
  };

  ws.onclose = () => {
    console.log('WebSocket closed, reconnecting in 3s...');
    setTimeout(connectWebSocket, 3000);
  };
}

function addLiveEvent(data) {
  const feed = document.getElementById('event-feed');
  const item = document.createElement('div');
  item.className = 'event-item';
  item.innerHTML = `
    <span class="ev-cam">${data.device_id}</span>
    <span class="ev-time">${new Date(data.timestamp).toLocaleTimeString()}</span>
    <span> — image captured → </span>
    <a href="${data.image_url}" target="_blank" style="color:#4ade80;font-size:11px">View</a>
  `;
  feed.prepend(item);

  // Keep only last 20 events
  while (feed.children.length > 20) feed.removeChild(feed.lastChild);
}