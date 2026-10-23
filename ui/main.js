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
  const invoke = window.__TAURI__?.core?.invoke;
  if (!invoke) return;
  invoke('generate_pairing_code').then(pin => window.alert(`Enter this code on the other DeskLink device: ${pin}`));
});
let sharing = false;
document.querySelector('#share-input').addEventListener('click', async (event) => {
  sharing = !sharing;
  try {
    const status = await window.__TAURI__.core.invoke('set_input_sharing', { enabled: sharing });
    event.currentTarget.textContent = sharing ? 'Stop keyboard and mouse sharing' : 'Start keyboard and mouse sharing';
    document.querySelector('#runtime-status').textContent = status;
  } catch (error) { sharing = false; document.querySelector('#runtime-status').textContent = `Could not change sharing: ${error}`; }
});
loadRuntimeStatus();

async function refreshPeers() {
  const list = document.querySelector('#peer-list');
  const label = document.querySelector('#peer-status');
  try {
    const peers = await window.__TAURI__.core.invoke('discovered_peers');
    label.textContent = peers.length ? `${peers.length} device(s) found` : 'Searching this Wi‑Fi…';
    list.innerHTML = peers.map(p => {
      const host = p.addresses[0];
      const address = host.includes(':') ? `[${host}]:${p.port}` : `${host}:${p.port}`;
      return `<p><strong>${p.name}</strong><br><small>${p.addresses.join(', ')}:${p.port}</small><br><button class="connect" data-address="${address}">Enter code and pair</button></p>`;
    }).join('');
    document.querySelectorAll('.connect').forEach(button => button.addEventListener('click', async () => {
      const pin = window.prompt('Enter the six-digit code shown on the other device');
      if (!pin) return;
      try {
        const result = await window.__TAURI__.core.invoke('pair_with_peer', { address: button.dataset.address, pin });
        button.textContent = result;
      } catch (error) { button.textContent = `Pairing failed: ${error}`; }
    }));
  } catch { label.textContent = 'Discovery is available in the desktop app.'; }
}
setInterval(refreshPeers, 1500); refreshPeers();

async function refreshPaired() {
  try {
    const peers = await window.__TAURI__.core.invoke('active_peers');
    const list = document.querySelector('#paired-list');
    list.innerHTML = peers.length ? peers.map(name => `<p><strong>${name}</strong><br><button class="send-test" data-peer="${name}">Send encrypted test input</button></p>`).join('') : 'No active pairings';
    document.querySelectorAll('.send-test').forEach(button => button.addEventListener('click', async () => {
      try { button.textContent = await window.__TAURI__.core.invoke('send_test_input', { peerName: button.dataset.peer }); }
      catch (error) { button.textContent = `Send failed: ${error}`; }
    }));
    const inputStatus = await window.__TAURI__.core.invoke('last_input_status');
    document.querySelector('#runtime-status').textContent = inputStatus;
  } catch { /* preview mode */ }
}
setInterval(refreshPaired, 1500); refreshPaired();
