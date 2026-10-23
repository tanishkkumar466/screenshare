const status = document.querySelector('#runtime-status');
async function loadRuntimeStatus() {
  try {
    const invoke = window.__TAURI__?.core?.invoke;
    if (!invoke) throw new Error('Desktop bridge unavailable');
    status.textContent = await invoke('runtime_status');
  } catch {
    status.textContent = 'DeskLink UI preview — start the desktop app to pair devices.';
  }
}
document.querySelector('#pair').addEventListener('click', () => {
  window.alert('Pairing will request a six-digit PIN from the other DeskLink device. This is the next networking milestone.');
});
loadRuntimeStatus();

async function refreshPeers() {
  const list = document.querySelector('#peer-list');
  const label = document.querySelector('#peer-status');
  try {
    const peers = await window.__TAURI__.core.invoke('discovered_peers');
    label.textContent = peers.length ? `${peers.length} device(s) found` : 'Searching this Wi‑Fi…';
    list.innerHTML = peers.map(p => `<p><strong>${p.name}</strong><br><small>${p.addresses.join(', ')}:${p.port}</small></p>`).join('');
  } catch { label.textContent = 'Discovery is available in the desktop app.'; }
}
setInterval(refreshPeers, 1500); refreshPeers();
