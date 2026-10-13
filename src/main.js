import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen, TauriEvent } from '@tauri-apps/api/event';
import { Icons } from './icons.js';

// State
let wallpapers = [];
let installingItems = [];
let activeWallpaper = null;
let isPlaying = false;
let isMuted = true;
let currentVolume = 0;
let currentSpeed = 1.0;
let currentQuality = 3;
let currentFps = 30;
let displays = [];
let steamStatus = { is_connected: false, user_name: 'Checking...', steam_id: '0', app_id: 480 };
let searchQuery = '';

// Quality Profile Definitions
const QUALITY_PROFILES = {
  1: {
    badge: 'Q1',
    title: 'Level 1: Max Performance',
    desc: 'Aggressive compression (720p, 24 FPS, CRF 28). Minimal CPU/GPU draw & battery saver.',
  },
  2: {
    badge: 'Q2',
    title: 'Level 2: Economy',
    desc: 'Optimized 1080p at 30 FPS (CRF 25). Efficient background playback for everyday use.',
  },
  3: {
    badge: 'Q3',
    title: 'Level 3: Balanced',
    desc: 'Crisp 1080p, 30 FPS, CRF 22 (Recommended). Balanced fidelity and low overhead.',
  },
  4: {
    badge: 'Q4',
    title: 'Level 4: High Fidelity',
    desc: 'High bitrate 1440p/4K, 60 FPS, CRF 19. Maximum clarity for gaming rigs.',
  },
  5: {
    badge: 'Q5',
    title: 'Level 5: Raw / Uncompressed',
    desc: 'Untouched source resolution, native frame rate and original bitrate.',
  },
};

// DOM Elements
const tabBtns = document.querySelectorAll('.nav-tab');
const tabContents = {
  library: document.getElementById('tab-library'),
  workshop: document.getElementById('tab-workshop'),
  displays: document.getElementById('tab-displays'),
  settings: document.getElementById('tab-settings'),
};

const libraryGrid = document.getElementById('library-grid');
const workshopGrid = document.getElementById('workshop-grid');
const displaysList = document.getElementById('displays-list');
const displayCount = document.getElementById('display-count');
const searchLibraryInput = document.getElementById('search-library');

// Media Bar Elements
const nowPlayingThumb = document.getElementById('now-playing-thumb');
const nowPlayingTitle = document.getElementById('now-playing-title');
const nowPlayingSub = document.getElementById('now-playing-sub');
const btnTogglePlay = document.getElementById('btn-toggle-play');
const btnStopWp = document.getElementById('btn-stop-wp');
const btnToggleMute = document.getElementById('btn-toggle-mute');
const sliderVolume = document.getElementById('slider-volume');
const selectSpeed = document.getElementById('select-speed');
const sliderQuality = document.getElementById('slider-quality');
const labelQualityVal = document.getElementById('label-quality-val');

// Settings Elements
const settingQualitySlider = document.getElementById('setting-quality-slider');
const settingQualityTitle = document.getElementById('setting-quality-title');
const settingQualityDesc = document.getElementById('setting-quality-desc');
const settingFpsSelect = document.getElementById('setting-fps-select');
const btnCompressActive = document.getElementById('btn-compress-active');
const btnCompressAll = document.getElementById('btn-compress-all');
const chkPauseFullscreen = document.getElementById('chk-pause-fullscreen');
const chkHardwareAccel = document.getElementById('chk-hardware-accel');

// Steam Status Header
const steamDot = document.getElementById('steam-dot');
const steamUserLabel = document.getElementById('steam-user-label');
const btnAddSteam = document.getElementById('btn-add-steam');
const btnImport = document.getElementById('btn-import');
const dropzone = document.getElementById('dropzone');

// Helper to convert or extract thumbnail URL (prioritizes base64 data for WebKitGTK)
function getThumbnail(wp) {
  if (!wp) return '/src-tauri/icons/icon.png';
  if (wp.thumbnail_data && wp.thumbnail_data.length > 0) {
    return wp.thumbnail_data;
  }
  if (wp.thumbnail_path) {
    if (
      wp.thumbnail_path.startsWith('http://') ||
      wp.thumbnail_path.startsWith('https://') ||
      wp.thumbnail_path.startsWith('data:')
    ) {
      return wp.thumbnail_path;
    }
    return convertFileSrc(wp.thumbnail_path);
  }
  return '/src-tauri/icons/icon.png';
}

// Populate Static Icons
function renderStaticIcons() {
  const setIcon = (id, iconHtml) => {
    const el = document.getElementById(id);
    if (el) el.innerHTML = iconHtml;
  };

  setIcon('icon-steam-slot', Icons.steam);
  setIcon('icon-import-slot', Icons.plus);
  setIcon('icon-tab-library', Icons.library);
  setIcon('icon-tab-workshop', Icons.workshop);
  setIcon('icon-tab-displays', Icons.displays);
  setIcon('icon-tab-settings', Icons.settings);
  setIcon('icon-search-slot', Icons.search);
  setIcon('icon-dropzone-slot', Icons.upload);
  setIcon('icon-publish-slot', Icons.rocket);
  setIcon('icon-slider-volume-slot', Icons.volumeHigh);
  setIcon('icon-slider-speed-slot', Icons.speed);
  setIcon('icon-slider-quality-slot', Icons.sliders);
  setIcon('icon-compress-active-slot', Icons.zap);
  setIcon('icon-compress-all-slot', Icons.gauge);

  btnTogglePlay.innerHTML = Icons.play;
  btnStopWp.innerHTML = Icons.stop;
  btnToggleMute.innerHTML = Icons.volumeMute;
}

// Initialize
async function init() {
  renderStaticIcons();
  setupTabs();
  setupSearch();
  setupMediaControls();
  setupPerformanceControls();
  setupImportAndDrop();
  setupSteamActions();

  await refreshSteamStatus();
  await refreshDisplays();
  await loadPerformanceSettings();
  await loadLibrary();
  await loadWorkshop();
}

// Tab Switching
function setupTabs() {
  tabBtns.forEach((btn) => {
    btn.addEventListener('click', () => {
      tabBtns.forEach((b) => b.classList.remove('active'));
      btn.classList.add('active');

      const targetTab = btn.getAttribute('data-tab');
      Object.keys(tabContents).forEach((tab) => {
        if (tab === targetTab) {
          tabContents[tab].classList.remove('hidden');
        } else {
          tabContents[tab].classList.add('hidden');
        }
      });
    });
  });
}

// Search Filter
function setupSearch() {
  if (searchLibraryInput) {
    searchLibraryInput.addEventListener('input', (e) => {
      searchQuery = e.target.value.toLowerCase().trim();
      renderLibrary();
    });
  }
}

// Steam Status Check
async function refreshSteamStatus() {
  try {
    steamStatus = await invoke('get_steam_status');
    if (steamStatus.is_connected) {
      steamDot.classList.remove('offline');
      steamUserLabel.textContent = `Steam: ${steamStatus.user_name} (${steamStatus.app_id})`;
    } else {
      steamDot.classList.add('offline');
      steamUserLabel.textContent = `Desktop Mode (Steam Offline)`;
    }
  } catch (e) {
    console.error('Steam status check error:', e);
    steamDot.classList.add('offline');
    steamUserLabel.textContent = 'Desktop Mode';
  }
}

// Displays
async function refreshDisplays() {
  try {
    displays = await invoke('get_monitors');
    displayCount.textContent = displays.length;
    renderDisplays();
  } catch (e) {
    console.error('Failed to get monitors:', e);
  }
}

function renderDisplays() {
  displaysList.innerHTML = displays
    .map(
      (d) => `
    <div style="background: var(--bg-card); border: 1px solid var(--border-card); border-radius: var(--radius-md); padding: 16px 20px; display: flex; justify-content: space-between; align-items: center;">
      <div style="display: flex; align-items: center; gap: 14px;">
        <div style="color: var(--text-muted);">${Icons.displays}</div>
        <div>
          <div style="font-weight: 500; font-size: 0.92rem; color: var(--text-main);">${d.name}</div>
          <div style="color: var(--text-muted); font-size: 0.78rem; margin-top: 2px;">Resolution: ${d.resolution} • ID: ${d.id}</div>
        </div>
      </div>
      <button class="btn btn-primary" onclick="window.applyToDisplay(${d.id})">
        Apply Wallpaper
      </button>
    </div>
  `
    )
    .join('');
}

window.applyToDisplay = async (screenId) => {
  if (!activeWallpaper) {
    alert('Please select a wallpaper first.');
    return;
  }
  await applyWallpaper(activeWallpaper, screenId);
};

// Library Wallpapers
async function loadLibrary() {
  try {
    wallpapers = await invoke('get_wallpapers');
    renderLibrary();
  } catch (e) {
    console.error('Failed to load wallpapers:', e);
  }
}

function renderLibrary() {
  const combined = [...installingItems, ...wallpapers];
  const filtered = combined.filter((wp) => {
    if (!searchQuery) return true;
    return wp.title.toLowerCase().includes(searchQuery);
  });

  if (filtered.length === 0) {
    libraryGrid.innerHTML = `
      <div style="grid-column: 1 / -1; padding: 48px 20px; text-align: center; color: var(--text-muted);">
        <div style="display: flex; justify-content: center; margin-bottom: 12px; color: var(--text-dim);">${Icons.library}</div>
        <p style="font-size: 0.95rem; font-weight: 400; color: var(--text-main); margin-bottom: 4px;">
          ${searchQuery ? 'No matching wallpapers found' : 'No animated wallpapers in your library yet'}
        </p>
        <p style="font-size: 0.8rem; color: var(--text-muted);">
          ${searchQuery ? 'Try adjusting your search query' : 'Drag & drop a video above or use "Import Video" to get started'}
        </p>
      </div>
    `;
    return;
  }

  libraryGrid.innerHTML = filtered
    .map((wp) => {
      const isInstalling = wp.isInstalling === true;
      const isActive = activeWallpaper && activeWallpaper.id === wp.id;
      const thumbSrc = getThumbnail(wp);

      if (isInstalling) {
        return `
        <div class="card" id="wp-card-${wp.id}">
          <div class="card-thumbnail-wrapper is-installing">
            <img src="${thumbSrc}" class="card-thumb" alt="${wp.title}" />
            <span class="badge-installing">Installing...</span>
            <div class="loading-bar-wrapper">
              <div class="loading-bar-line"></div>
            </div>
          </div>
          <div class="card-body">
            <div class="card-title">${wp.title}</div>
            <div class="card-meta">Processing video & extracting frames...</div>
            <div class="card-footer">
              <button class="btn btn-primary" disabled style="opacity: 0.6; cursor: wait;">
                ${Icons.spinner} Installing
              </button>
            </div>
          </div>
        </div>
      `;
      }

      return `
      <div class="card ${isActive ? 'active-wallpaper' : ''}" id="wp-card-${wp.id}">
        <div class="card-thumbnail-wrapper" id="thumb-wrapper-${wp.id}">
          <img src="${thumbSrc}" class="card-thumb" id="thumb-img-${wp.id}" alt="${wp.title}" />
          ${isActive ? '<span class="badge-active">Playing</span>' : ''}
          <span class="badge-res">${wp.resolution || 'HD'}</span>
        </div>
        <div class="card-body">
          <div class="card-title editable" id="wp-title-${wp.id}" ondblclick="window.onStartRename('${wp.id}')" title="Double-click to rename">
            ${wp.title}
          </div>
          <div class="card-meta">Format: ${wp.format.toUpperCase()} • ${Math.round(wp.duration_secs)}s</div>
          <div class="card-footer">
            <button class="btn ${isActive ? 'btn-success' : 'btn-primary'}" onclick="window.onApplyWallpaper('${wp.id}')">
              ${isActive ? `${Icons.check} Active` : `${Icons.play} Apply`}
            </button>
            <div style="display: flex; gap: 4px;">
              <button class="btn btn-icon" style="color: var(--text-muted);" onclick="window.onOptimizeWallpaper('${wp.id}')" title="Optimize / Compress">
                ${Icons.zap}
              </button>
              <button class="btn btn-icon" style="color: var(--text-muted);" onclick="window.onDeleteWallpaper('${wp.id}')" title="Delete">
                ${Icons.trash}
              </button>
            </div>
          </div>
        </div>
      </div>
    `;
    })
    .join('');
}

// Double-Click Inline Rename
window.onStartRename = (id) => {
  const wp = wallpapers.find((w) => w.id === id);
  if (!wp) return;
  const titleEl = document.getElementById(`wp-title-${id}`);
  if (!titleEl || titleEl.querySelector('input')) return;

  const currentTitle = wp.title;
  titleEl.innerHTML = `<input type="text" class="rename-input" id="rename-input-${id}" value="${currentTitle}" spellcheck="false" />`;
  const input = document.getElementById(`rename-input-${id}`);
  if (!input) return;

  input.focus();
  input.select();

  let committed = false;
  const finish = async (save) => {
    if (committed) return;
    committed = true;
    const newTitle = input.value.trim();
    if (save && newTitle && newTitle !== currentTitle) {
      try {
        const updated = await invoke('rename_wallpaper', { id, newTitle });
        wp.title = updated.title;
        titleEl.textContent = updated.title;
        titleEl.title = 'Double-click to rename';
        if (activeWallpaper && activeWallpaper.id === id) {
          activeWallpaper.title = updated.title;
          nowPlayingTitle.textContent = updated.title;
        }
      } catch (err) {
        console.error('Failed to rename:', err);
        titleEl.textContent = currentTitle;
      }
    } else {
      titleEl.textContent = currentTitle;
    }
  };

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      finish(true);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      finish(false);
    }
  });

  input.addEventListener('blur', () => {
    finish(true);
  });
};

window.onApplyWallpaper = async (id) => {
  const wp = wallpapers.find((w) => w.id === id);
  if (wp) {
    await applyWallpaper(wp, null);
  }
};

window.onDeleteWallpaper = async (id) => {
  if (confirm('Are you sure you want to delete this wallpaper?')) {
    try {
      await invoke('delete_wallpaper', { id });
      await loadLibrary();
    } catch (e) {
      alert('Error deleting wallpaper: ' + e);
    }
  }
};

window.onOptimizeWallpaper = async (id) => {
  const wp = wallpapers.find((w) => w.id === id);
  if (!wp) return;

  // Add installing / optimizing feedback to card
  const cardThumbWrapper = document.getElementById(`thumb-wrapper-${id}`);
  if (cardThumbWrapper) {
    cardThumbWrapper.classList.add('is-installing');
    const loadingLine = document.createElement('div');
    loadingLine.className = 'loading-bar-wrapper';
    loadingLine.innerHTML = '<div class="loading-bar-line"></div>';
    cardThumbWrapper.appendChild(loadingLine);
  }

  try {
    const updated = await invoke('compress_wallpaper', {
      id,
      qualityLevel: currentQuality,
      targetFps: currentFps,
    });
    // Replace in array
    const idx = wallpapers.findIndex((w) => w.id === id);
    if (idx !== -1) {
      wallpapers[idx] = updated;
    }
    if (activeWallpaper && activeWallpaper.id === id) {
      activeWallpaper = updated;
    }
    // Smooth unblur transition
    if (cardThumbWrapper) {
      const img = document.getElementById(`thumb-img-${id}`);
      if (img) img.src = getThumbnail(updated);
      cardThumbWrapper.classList.remove('is-installing');
      const bar = cardThumbWrapper.querySelector('.loading-bar-wrapper');
      if (bar) bar.remove();
    }
    setTimeout(() => renderLibrary(), 600);
  } catch (err) {
    alert('Optimization error: ' + err);
    if (cardThumbWrapper) {
      cardThumbWrapper.classList.remove('is-installing');
      const bar = cardThumbWrapper.querySelector('.loading-bar-wrapper');
      if (bar) bar.remove();
    }
  }
};

async function applyWallpaper(wp, screenId) {
  try {
    await invoke('apply_wallpaper', {
      wallpaper: wp,
      screen: screenId !== null ? screenId : null,
    });
    activeWallpaper = wp;
    isPlaying = true;

    // Update bottom player
    nowPlayingThumb.src = getThumbnail(wp);
    nowPlayingTitle.textContent = wp.title;
    nowPlayingSub.textContent = `Playing on Desktop (${wp.format.toUpperCase()} • ${wp.resolution || 'HD'})`;
    btnTogglePlay.innerHTML = Icons.pause;

    renderLibrary();
  } catch (e) {
    alert('Failed to set wallpaper: ' + e);
  }
}

// Media Controls
function setupMediaControls() {
  btnTogglePlay.addEventListener('click', async () => {
    if (!activeWallpaper) return;
    isPlaying = !isPlaying;
    btnTogglePlay.innerHTML = isPlaying ? Icons.pause : Icons.play;
    try {
      await invoke('set_playback_state', {
        isPaused: !isPlaying,
        volume: parseInt(sliderVolume.value),
        isMuted: isMuted,
        speed: parseFloat(selectSpeed.value),
      });
      nowPlayingSub.textContent = isPlaying ? 'Playing on Desktop' : 'Paused';
    } catch (e) {
      console.error(e);
    }
  });

  btnStopWp.addEventListener('click', async () => {
    try {
      await invoke('stop_wallpaper');
      activeWallpaper = null;
      isPlaying = false;
      nowPlayingTitle.textContent = 'No Wallpaper Active';
      nowPlayingSub.textContent = 'Desktop restored to default';
      btnTogglePlay.innerHTML = Icons.play;
      renderLibrary();
    } catch (e) {
      console.error(e);
    }
  });

  btnToggleMute.addEventListener('click', async () => {
    isMuted = !isMuted;
    btnToggleMute.innerHTML = isMuted ? Icons.volumeMute : Icons.volumeHigh;
    try {
      await invoke('set_playback_state', {
        isPaused: !isPlaying,
        volume: parseInt(sliderVolume.value),
        isMuted: isMuted,
        speed: parseFloat(selectSpeed.value),
      });
    } catch (e) {
      console.error(e);
    }
  });

  sliderVolume.addEventListener('input', async (e) => {
    currentVolume = parseInt(e.target.value);
    if (currentVolume > 0 && isMuted) {
      isMuted = false;
      btnToggleMute.innerHTML = Icons.volumeHigh;
    }
    try {
      await invoke('set_playback_state', {
        isPaused: !isPlaying,
        volume: currentVolume,
        isMuted: isMuted,
        speed: parseFloat(selectSpeed.value),
      });
    } catch (err) {
      console.error(err);
    }
  });

  selectSpeed.addEventListener('change', async (e) => {
    currentSpeed = parseFloat(e.target.value);
    try {
      await invoke('set_playback_state', {
        isPaused: !isPlaying,
        volume: currentVolume,
        isMuted: isMuted,
        speed: currentSpeed,
      });
    } catch (err) {
      console.error(err);
    }
  });

  // Bottom Media Bar Quality Slider (1 to 5)
  sliderQuality.addEventListener('input', async (e) => {
    const val = parseInt(e.target.value);
    applyQualityLevel(val);
    await syncPerformanceSettings();
  });
}

// Performance & Quality Settings Controls
function setupPerformanceControls() {
  settingQualitySlider.addEventListener('input', async (e) => {
    const val = parseInt(e.target.value);
    applyQualityLevel(val);
    await syncPerformanceSettings();
  });

  settingFpsSelect.addEventListener('change', async (e) => {
    currentFps = parseInt(e.target.value);
    await syncPerformanceSettings();
  });

  chkPauseFullscreen.addEventListener('change', async () => {
    await syncPerformanceSettings();
  });

  chkHardwareAccel.addEventListener('change', async () => {
    await syncPerformanceSettings();
  });

  // Optimize Active Wallpaper button
  btnCompressActive.addEventListener('click', async () => {
    if (!activeWallpaper) {
      alert('Please select or play a wallpaper first to optimize it.');
      return;
    }
    btnCompressActive.disabled = true;
    const origHtml = btnCompressActive.innerHTML;
    btnCompressActive.innerHTML = `${Icons.spinner} <span>Compressing Video...</span>`;

    try {
      const updated = await invoke('compress_wallpaper', {
        id: activeWallpaper.id,
        qualityLevel: currentQuality,
        targetFps: currentFps,
      });
      activeWallpaper = updated;
      const idx = wallpapers.findIndex((w) => w.id === updated.id);
      if (idx !== -1) wallpapers[idx] = updated;
      renderLibrary();
      alert(`Wallpaper successfully compressed to Level ${currentQuality} (${updated.resolution}). File footprint and GPU decoding overhead reduced.`);
    } catch (err) {
      alert('Compression error: ' + err);
    } finally {
      btnCompressActive.disabled = false;
      btnCompressActive.innerHTML = origHtml;
    }
  });

  // Compress Entire Library button
  btnCompressAll.addEventListener('click', async () => {
    if (wallpapers.length === 0) {
      alert('No wallpapers in library to compress.');
      return;
    }
    if (!confirm(`Compress all ${wallpapers.length} wallpapers in your library to Level ${currentQuality} (${currentFps} FPS)?`)) {
      return;
    }

    btnCompressAll.disabled = true;
    const origHtml = btnCompressAll.innerHTML;
    btnCompressAll.innerHTML = `${Icons.spinner} <span>Compressing Library...</span>`;

    let count = 0;
    for (const wp of wallpapers) {
      try {
        const updated = await invoke('compress_wallpaper', {
          id: wp.id,
          qualityLevel: currentQuality,
          targetFps: currentFps,
        });
        const idx = wallpapers.findIndex((w) => w.id === updated.id);
        if (idx !== -1) wallpapers[idx] = updated;
        count++;
      } catch (e) {
        console.error('Failed to compress ' + wp.title, e);
      }
    }

    btnCompressAll.disabled = false;
    btnCompressAll.innerHTML = origHtml;
    renderLibrary();
    alert(`Successfully optimized ${count} wallpaper(s) in your library.`);
  });
}

function applyQualityLevel(val) {
  currentQuality = val;
  sliderQuality.value = val;
  settingQualitySlider.value = val;

  const profile = QUALITY_PROFILES[val] || QUALITY_PROFILES[3];
  labelQualityVal.textContent = profile.badge;
  settingQualityTitle.textContent = profile.title;
  settingQualityDesc.textContent = profile.desc;
}

async function loadPerformanceSettings() {
  try {
    const s = await invoke('get_performance_settings');
    currentQuality = s.quality_level || 3;
    currentFps = s.target_fps || 30;
    applyQualityLevel(currentQuality);
    settingFpsSelect.value = currentFps.toString();
    chkPauseFullscreen.checked = s.pause_on_fullscreen;
    chkHardwareAccel.checked = s.hardware_accel;
  } catch (e) {
    console.error('Failed to load performance settings:', e);
  }
}

async function syncPerformanceSettings() {
  try {
    await invoke('update_performance_settings', {
      qualityLevel: currentQuality,
      targetFps: currentFps,
      pauseOnFullscreen: chkPauseFullscreen.checked,
      hardwareAccel: chkHardwareAccel.checked,
    });
  } catch (err) {
    console.error('Failed to sync performance settings:', err);
  }
}

// Import & Drag Drop with QOL Blurred Preview + Slick White Loading Line
function setupImportAndDrop() {
  async function triggerNativeFilePicker() {
    try {
      btnImport.disabled = true;
      btnImport.innerHTML = `${Icons.spinner} <span>Importing...</span>`;
      const imported = await invoke('select_and_import_videos');
      if (imported && imported.length > 0) {
        await loadLibrary();
      }
    } catch (e) {
      alert('Error importing video: ' + e);
    } finally {
      btnImport.disabled = false;
      btnImport.innerHTML = `${Icons.plus} <span>Import Video</span>`;
    }
  }

  btnImport.addEventListener('click', triggerNativeFilePicker);
  dropzone.addEventListener('click', triggerNativeFilePicker);

  listen(TauriEvent.DRAG_ENTER, () => {
    dropzone.classList.add('dragover');
  });

  listen(TauriEvent.DRAG_LEAVE, () => {
    dropzone.classList.remove('dragover');
  });

  listen(TauriEvent.DRAG_DROP, async (event) => {
    dropzone.classList.remove('dragover');
    const paths = event.payload?.paths;
    if (paths && paths.length > 0) {
      // Create optimistic installing items with blurred placeholders and white loading lines
      const addedTemp = [];
      for (const p of paths) {
        const filename = p.split('/').pop().replace(/\.[^/.]+$/, '');
        const tempItem = {
          id: 'temp_' + Date.now() + '_' + Math.random().toString(36).slice(2, 6),
          title: filename,
          thumbnail_data: '/src-tauri/icons/icon.png',
          format: p.split('.').pop() || 'mp4',
          duration_secs: 0,
          resolution: 'Importing...',
          isInstalling: true,
        };
        installingItems.unshift(tempItem);
        addedTemp.push({ path: p, tempId: tempItem.id });
      }
      renderLibrary();

      // Sequentially import and unblur
      for (const item of addedTemp) {
        try {
          const imported = await invoke('import_wallpaper', { sourcePath: item.path });
          // Remove from installing list
          installingItems = installingItems.filter((i) => i.id !== item.tempId);
          wallpapers.unshift(imported);

          // Find the card element to trigger smooth CSS unblur
          const cardEl = document.getElementById(`wp-card-${item.tempId}`);
          if (cardEl) {
            const thumb = cardEl.querySelector('.card-thumb');
            const wrapper = cardEl.querySelector('.card-thumbnail-wrapper');
            const bar = cardEl.querySelector('.loading-bar-wrapper');
            const badge = cardEl.querySelector('.badge-installing');
            if (thumb) thumb.src = getThumbnail(imported);
            if (wrapper) wrapper.classList.remove('is-installing');
            if (bar) bar.remove();
            if (badge) badge.remove();
          }
        } catch (err) {
          console.error('Failed to import ' + item.path, err);
          installingItems = installingItems.filter((i) => i.id !== item.tempId);
        }
      }

      setTimeout(() => renderLibrary(), 600);
    }
  });

  dropzone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropzone.classList.add('dragover');
  });

  dropzone.addEventListener('dragleave', () => {
    dropzone.classList.remove('dragover');
  });
}

// Workshop Tab
async function loadWorkshop() {
  try {
    const items = await invoke('get_workshop_items');
    renderWorkshop(items);
  } catch (e) {
    console.error('Failed to load workshop items:', e);
  }
}

function renderWorkshop(items) {
  workshopGrid.innerHTML = items
    .map(
      (item) => `
    <div class="card">
      <div class="card-thumbnail-wrapper">
        <img src="${item.preview_url}" class="card-thumb" alt="${item.title}" />
        <span class="badge-res">${item.file_size_mb} MB</span>
      </div>
      <div class="card-body">
        <div class="card-title">${item.title}</div>
        <div class="card-desc">${item.description}</div>
        <div class="card-meta">By ${item.author} • Upvotes: ${item.votes_up}</div>
        <div class="card-footer">
          <button class="btn btn-primary" onclick="window.downloadWorkshopItem('${item.id}')">
            ${Icons.download} Subscribe
          </button>
        </div>
      </div>
    </div>
  `
    )
    .join('');
}

// Workshop Download with QOL Blurred Preview + White Scanning Progress Bar
window.downloadWorkshopItem = async (id) => {
  const workshopItems = await invoke('get_workshop_items').catch(() => []);
  const item = workshopItems.find((w) => w.id === id);
  if (!item) return;

  // Switch to Library tab immediately so user sees the download progress
  const tabBtnLib = document.getElementById('tab-btn-library');
  if (tabBtnLib) tabBtnLib.click();

  const tempId = 'ws_install_' + Date.now();
  const installingEntry = {
    id: tempId,
    title: item.title,
    thumbnail_data: item.preview_url,
    thumbnail_path: item.preview_url,
    format: 'mp4',
    duration_secs: 15,
    resolution: '1080p',
    isInstalling: true,
  };
  installingItems.unshift(installingEntry);
  renderLibrary();

  // Simulate smooth network stream & video registration
  setTimeout(async () => {
    installingItems = installingItems.filter((i) => i.id !== tempId);

    const installed = {
      id: 'wp_' + tempId,
      title: item.title,
      file_path: '/home/martin/Main/Programming/AnimatedWallpaper/assets/sample_wallpapers/cyber_matrix.mp4',
      thumbnail_path: item.preview_url,
      thumbnail_data: item.preview_url,
      duration_secs: 12.0,
      resolution: '1920x1080',
      format: 'mp4',
      is_workshop: true,
      workshop_id: id,
    };
    wallpapers.unshift(installed);

    // Smooth unblur transition
    const cardEl = document.getElementById(`wp-card-${tempId}`);
    if (cardEl) {
      const wrapper = cardEl.querySelector('.card-thumbnail-wrapper');
      const bar = cardEl.querySelector('.loading-bar-wrapper');
      const badge = cardEl.querySelector('.badge-installing');
      if (wrapper) wrapper.classList.remove('is-installing');
      if (bar) bar.remove();
      if (badge) badge.remove();
    }

    setTimeout(() => renderLibrary(), 600);
  }, 1600);
};

// Steam Actions
function setupSteamActions() {
  btnAddSteam.addEventListener('click', async () => {
    try {
      const res = await invoke('add_to_steam_client');
      alert(res);
    } catch (e) {
      alert('Steam shortcut error: ' + e);
    }
  });
}

// Start Application
init();
