import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen, TauriEvent } from '@tauri-apps/api/event';
import { Icons } from './icons.js';

// State
let wallpapers = [];
let installingItems = [];
let activeWallpaper = null;
let isPlaying = false;
let currentSpeed = 1.0;
let currentQuality = 5;
let currentFps = 30;
let currentFillMode = 2; // 0: Stretch, 1: Fit, 2: Crop/Zoom, 3: Center
let currentPanX = 0.0;
let currentPanY = 0.0;
let currentPausePolicy = 1; // 0: Never, 1: Fullscreen Game/App, 2: Maximized Window, 3: Any Window, 4: Battery
let displays = [];
let searchQuery = '';
let currentSort = 'recent';
let pendingDeleteId = null;

// Quality Profile Definitions
const QUALITY_PROFILES = {
  5: {
    badge: 'Q5',
    title: 'Q5: Native (Source Resolution)',
    desc: 'Untouched source video preserving native frame rate, resolution, and original bitrate.',
  },
  4: {
    badge: 'Q4',
    title: 'Q4: 4K (3840x2160)',
    desc: 'Ultra high-definition 4K encoding at 60 FPS (CRF 18). Crystal clear for UHD displays.',
  },
  3: {
    badge: 'Q3',
    title: 'Q3: 2K (2560x1440)',
    desc: 'Quad HD 1440p encoding at 60 FPS (CRF 21). Ideal balance for 1440p monitors.',
  },
  2: {
    badge: 'Q2',
    title: 'Q2: FULLHD (1920x1080)',
    desc: 'Standard Full HD 1080p at 30 FPS (CRF 24). Low GPU overhead and great efficiency.',
  },
  1: {
    badge: 'Q1',
    title: 'Q1: HD or lower (1280x720)',
    desc: 'Downscaled 720p at 24 FPS (CRF 28). Minimal CPU/GPU draw & maximum battery saving.',
  },
};

// DOM Elements
const tabBtns = document.querySelectorAll('.nav-tab');
const tabContents = {
  library: document.getElementById('tab-library'),
  displays: document.getElementById('tab-displays'),
  settings: document.getElementById('tab-settings'),
};

const libraryGrid = document.getElementById('library-grid');
const displaysList = document.getElementById('displays-list');
const displayCount = document.getElementById('display-count');
const searchLibraryInput = document.getElementById('search-library');
const libraryCountBadge = document.getElementById('library-count-badge');
const selectLibrarySort = document.getElementById('select-library-sort');
const btnClearSearch = document.getElementById('btn-clear-search');
const btnOpenFolder = document.getElementById('btn-open-folder');
const btnOpenFolderSettings = document.getElementById('btn-open-folder-settings');
const btnClearCache = document.getElementById('btn-clear-cache');
const playerInfoJump = document.getElementById('player-info-jump');

// Delete Modal Elements
const modalDeleteConfirm = document.getElementById('modal-delete-confirm');
const modalDeleteDesc = document.getElementById('modal-delete-desc');
const btnConfirmDelete = document.getElementById('btn-confirm-delete');
const btnCancelDelete = document.getElementById('btn-cancel-delete');
const btnCancelDeleteX = document.getElementById('btn-cancel-delete-x');

// Media Bar Elements
const nowPlayingThumb = document.getElementById('now-playing-thumb');
const nowPlayingTitle = document.getElementById('now-playing-title');
const nowPlayingSub = document.getElementById('now-playing-sub');
const btnTogglePlay = document.getElementById('btn-toggle-play');
const btnStopWp = document.getElementById('btn-stop-wp');
const selectSpeed = document.getElementById('select-speed');
const sliderQuality = document.getElementById('slider-quality');
const labelQualityVal = document.getElementById('label-quality-val');
const selectSizing = document.getElementById('select-sizing');
const selectPanX = document.getElementById('select-pan-x');
const selectPanY = document.getElementById('select-pan-y');
const btnResetPan = document.getElementById('btn-reset-pan');

// Settings Elements
const settingQualitySlider = document.getElementById('setting-quality-slider');
const settingQualityTitle = document.getElementById('setting-quality-title');
const settingQualityDesc = document.getElementById('setting-quality-desc');
const settingFpsSelect = document.getElementById('setting-fps-select');
const settingPausePolicy = document.getElementById('setting-pause-policy');
const btnCompressActive = document.getElementById('btn-compress-active');
const btnCompressAll = document.getElementById('btn-compress-all');
const chkHardwareAccel = document.getElementById('chk-hardware-accel');
const settingBackend = document.getElementById('setting-backend');
const chkAutostart = document.getElementById('chk-autostart');
const labelAutostartStatus = document.getElementById('label-autostart-status');

const btnImport = document.getElementById('btn-import');
const dropzone = document.getElementById('dropzone');

// Close Controls Dialog Elements
const modalCloseControls = document.getElementById('modal-close-controls');
const btnCloseDialogCancel = document.getElementById('btn-close-dialog-cancel');
const btnDialogCancelFooter = document.getElementById('btn-dialog-cancel-footer');
const btnOptTray = document.getElementById('btn-opt-tray');
const btnOptMinimize = document.getElementById('btn-opt-minimize');
const btnOptQuit = document.getElementById('btn-opt-quit');
const chkRememberCloseAction = document.getElementById('chk-remember-close-action');

const settingCloseBehavior = document.getElementById('setting-close-behavior');
const btnResetClosePref = document.getElementById('btn-reset-close-pref');

// Toast Notification Function
function showToast(message, type = 'info') {
  const container = document.getElementById('toast-container');
  if (!container) return;
  const toast = document.createElement('div');
  toast.className = `toast toast-${type}`;

  let iconSvg = Icons.zap;
  if (type === 'success') iconSvg = Icons.check;
  else if (type === 'error') iconSvg = Icons.x;
  else if (type === 'info') iconSvg = Icons.library;

  toast.innerHTML = `<span class="toast-icon">${iconSvg}</span><span>${message}</span>`;
  container.appendChild(toast);

  requestAnimationFrame(() => {
    toast.classList.add('show');
  });

  setTimeout(() => {
    toast.classList.remove('show');
    setTimeout(() => toast.remove(), 300);
  }, 3200);
}

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

  setIcon('icon-import-slot', Icons.plus);
  setIcon('icon-folder-slot', Icons.folder);
  setIcon('icon-folder-settings-slot', Icons.folder);
  setIcon('icon-clear-search-slot', Icons.x);
  setIcon('icon-tab-library', Icons.library);
  setIcon('icon-tab-displays', Icons.displays);
  setIcon('icon-tab-settings', Icons.settings);
  setIcon('icon-search-slot', Icons.search);
  setIcon('icon-dropzone-slot', Icons.upload);
  setIcon('icon-slider-speed-slot', Icons.speed);
  setIcon('icon-slider-quality-slot', Icons.sliders);
  setIcon('icon-sizing-slot', Icons.sizing);
  setIcon('icon-compress-active-slot', Icons.zap);
  setIcon('icon-compress-all-slot', Icons.gauge);

  setIcon('icon-win-min', Icons.winMin);
  setIcon('icon-win-max', Icons.winMax);
  setIcon('icon-win-close', Icons.winClose);

  btnTogglePlay.innerHTML = Icons.play;
  btnStopWp.innerHTML = Icons.stop;
}

// Initialize
async function init() {
  renderStaticIcons();
  setupTabs();
  setupSearch();
  setupMediaControls();
  setupGeometryControls();
  setupPerformanceControls();
  setupImportAndDrop();
  setupWindowControls();
  setupKeyboardShortcuts();
  setupStorageControls();
  setupAutostartControls();
  setupPlayerInfoJump();
  setupDeleteModal();

  await refreshDisplays();
  await loadPerformanceSettings();
  await loadLibrary();
  await refreshCacheStats();
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

// Search & Sort Controls
function setupSearch() {
  if (searchLibraryInput) {
    searchLibraryInput.addEventListener('input', (e) => {
      searchQuery = e.target.value.toLowerCase().trim();
      if (btnClearSearch) {
        if (e.target.value.length > 0) {
          btnClearSearch.classList.remove('hidden');
        } else {
          btnClearSearch.classList.add('hidden');
        }
      }
      renderLibrary();
    });
  }

  if (btnClearSearch) {
    btnClearSearch.addEventListener('click', () => {
      if (searchLibraryInput) {
        searchLibraryInput.value = '';
        searchQuery = '';
        searchLibraryInput.focus();
      }
      btnClearSearch.classList.add('hidden');
      renderLibrary();
    });
  }

  if (selectLibrarySort) {
    selectLibrarySort.addEventListener('change', (e) => {
      currentSort = e.target.value;
      renderLibrary();
    });
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
    showToast('Please select a wallpaper first', 'info');
    return;
  }
  await applyWallpaper(activeWallpaper, screenId);
  showToast(`Applied "${activeWallpaper.title}" to display ${screenId}`, 'success');
};

// Library Wallpapers
async function loadLibrary() {
  try {
    wallpapers = await invoke('get_wallpapers');
    try {
      const active = await invoke('get_active_wallpaper');
      if (active) {
        activeWallpaper = active;
        nowPlayingThumb.src = getThumbnail(active);
        nowPlayingTitle.textContent = active.title;
        nowPlayingSub.textContent = `Playing on Desktop (${active.format.toUpperCase()} • ${active.resolution || 'HD'})`;
        btnTogglePlay.innerHTML = Icons.pause;
        isPlaying = true;
      }
    } catch (_) {}
    renderLibrary();
    checkVariantCacheState();
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

  if (libraryCountBadge) {
    if (searchQuery) {
      libraryCountBadge.textContent = `${filtered.length} / ${combined.length}`;
    } else {
      libraryCountBadge.textContent = `${combined.length}`;
    }
  }

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

  const sorted = [...filtered];
  if (currentSort === 'title-asc') {
    sorted.sort((a, b) => a.title.localeCompare(b.title));
  } else if (currentSort === 'title-desc') {
    sorted.sort((a, b) => b.title.localeCompare(a.title));
  } else if (currentSort === 'duration') {
    sorted.sort((a, b) => (b.duration_secs || 0) - (a.duration_secs || 0));
  } else if (currentSort === 'res') {
    const parseRes = (r) => {
      const parts = (r || '').split('x').map((n) => parseInt(n) || 0);
      return (parts[0] || 0) * (parts[1] || 1);
    };
    sorted.sort((a, b) => parseRes(b.resolution) - parseRes(a.resolution));
  }

  libraryGrid.innerHTML = sorted
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

      const isCachedForLevel = (wp.cached_qualities || [5]).includes(currentQuality);
      const cachedVariants = (wp.cached_qualities || [5])
        .map((q) => `Q${q}`)
        .join(' ');

      return `
      <div class="card ${isActive ? 'active-wallpaper' : ''}" 
           id="wp-card-${wp.id}" 
           ondblclick="window.onApplyWallpaper('${wp.id}')" 
           title="Double-click to apply wallpaper">
        <div class="card-thumbnail-wrapper" id="thumb-wrapper-${wp.id}">
          <img src="${thumbSrc}" class="card-thumb" id="thumb-img-${wp.id}" alt="${wp.title}" />
          ${isActive ? '<span class="badge-active">Playing</span>' : ''}
          <span class="badge-res">${wp.resolution || 'HD'}</span>
        </div>
        <div class="card-body">
          <div class="card-title editable" 
               id="wp-title-${wp.id}" 
               ondblclick="event.stopPropagation(); window.onStartRename('${wp.id}')" 
               title="Double-click title to rename">
            ${wp.title}
          </div>
          <div class="card-meta">Format: ${wp.format.toUpperCase()} • ${Math.round(wp.duration_secs)}s • Cached: [${cachedVariants}]</div>
          <div class="card-footer">
            <button class="btn ${isActive ? 'btn-success' : 'btn-primary'}" onclick="event.stopPropagation(); window.onApplyWallpaper('${wp.id}')">
              ${isActive ? `${Icons.check} Active` : `${Icons.play} Apply`}
            </button>
            <div style="display: flex; gap: 4px;">
              <button class="btn btn-icon ${!isCachedForLevel ? 'btn-optimize-ready' : ''}" 
                      id="btn-opt-${wp.id}" 
                      onclick="event.stopPropagation(); window.onOptimizeWallpaper('${wp.id}')" 
                      title="${!isCachedForLevel ? `Optimize / Cache Q${currentQuality}` : `Variant Q${currentQuality} Cached`}">
                ${Icons.zap}
              </button>
              <button class="btn btn-icon" onclick="event.stopPropagation(); window.onDeleteWallpaper('${wp.id}')" title="Delete">
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
        titleEl.title = 'Double-click title to rename';
        if (activeWallpaper && activeWallpaper.id === id) {
          activeWallpaper.title = updated.title;
          nowPlayingTitle.textContent = updated.title;
        }
        showToast(`Renamed wallpaper to "${updated.title}"`, 'info');
      } catch (err) {
        showToast(`Failed to rename: ${err}`, 'error');
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
    showToast(`Applied "${wp.title}" to Desktop`, 'success');
  }
};

window.onDeleteWallpaper = (id) => {
  showDeleteConfirm(id);
};

window.onOptimizeWallpaper = async (id) => {
  const wp = wallpapers.find((w) => w.id === id);
  if (!wp) return;

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
    const idx = wallpapers.findIndex((w) => w.id === id);
    if (idx !== -1) {
      wallpapers[idx] = updated;
    }
    if (activeWallpaper && activeWallpaper.id === id) {
      activeWallpaper = updated;
      nowPlayingSub.textContent = `Playing on Desktop (${updated.format.toUpperCase()} • ${updated.resolution || 'HD'})`;
    }

    checkVariantCacheState();
    showToast(`Cached variant Q${currentQuality} for "${updated.title}"`, 'success');
    await refreshCacheStats();

    if (cardThumbWrapper) {
      const img = document.getElementById(`thumb-img-${id}`);
      if (img) img.src = getThumbnail(updated);
      cardThumbWrapper.classList.remove('is-installing');
      const bar = cardThumbWrapper.querySelector('.loading-bar-wrapper');
      if (bar) bar.remove();
    }
    setTimeout(() => renderLibrary(), 600);
  } catch (err) {
    showToast('Optimization error: ' + err, 'error');
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

    checkVariantCacheState();
    renderLibrary();
  } catch (e) {
    showToast('Failed to set wallpaper: ' + e, 'error');
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
        volume: 0,
        isMuted: true,
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
      checkVariantCacheState();
      renderLibrary();
    } catch (e) {
      console.error(e);
    }
  });

  selectSpeed.addEventListener('change', async (e) => {
    currentSpeed = parseFloat(e.target.value);
    try {
      await invoke('set_playback_state', {
        isPaused: !isPlaying,
        volume: 0,
        isMuted: true,
        speed: currentSpeed,
      });
    } catch (err) {
      console.error(err);
    }
  });

  // Bottom Media Bar Quality Slider (1 to 5)
  sliderQuality.addEventListener('input', async (e) => {
    const val = parseInt(e.target.value);
    await onQualitySliderChanged(val);
  });
}

// Helper to set or add formatted pan value (+20, -20, 0) in select dropdown
function setPanSelectValue(selectEl, numVal) {
  if (!selectEl) return;
  const rounded = Math.round(numVal * 100);
  const str = rounded.toString();
  const existing = Array.from(selectEl.options).find((opt) => opt.value === str);
  if (existing) {
    selectEl.value = str;
  } else {
    const label = rounded > 0 ? `+${rounded}` : `${rounded}`;
    const newOpt = new Option(label, str, true, true);
    selectEl.add(newOpt);
    selectEl.value = str;
  }
}

// Geometry / Sizing and Pan Controls
function setupGeometryControls() {
  selectSizing.addEventListener('change', async (e) => {
    currentFillMode = parseInt(e.target.value);
    await syncGeometry();
  });

  selectPanX.addEventListener('change', async (e) => {
    currentPanX = parseInt(e.target.value) / 100.0;
    await syncGeometry();
  });

  selectPanY.addEventListener('change', async (e) => {
    currentPanY = parseInt(e.target.value) / 100.0;
    await syncGeometry();
  });

  btnResetPan.addEventListener('click', async () => {
    currentPanX = 0.0;
    currentPanY = 0.0;
    selectPanX.value = '0';
    selectPanY.value = '0';
    await syncGeometry();
  });
}

async function syncGeometry() {
  try {
    await invoke('set_video_geometry', {
      fillMode: currentFillMode,
      panX: currentPanX,
      panY: currentPanY,
    });
  } catch (err) {
    console.error('Failed to set video geometry:', err);
  }
}

// Performance & Quality Settings Controls
function setupPerformanceControls() {
  settingQualitySlider.addEventListener('input', async (e) => {
    const val = parseInt(e.target.value);
    await onQualitySliderChanged(val);
  });

  settingFpsSelect.addEventListener('change', async (e) => {
    currentFps = parseInt(e.target.value);
    await syncPerformanceSettings();
  });

  settingPausePolicy.addEventListener('change', async (e) => {
    currentPausePolicy = parseInt(e.target.value);
    await syncPerformanceSettings();
  });

  chkHardwareAccel.addEventListener('change', async () => {
    await syncPerformanceSettings();
  });

  // Optimize Active Wallpaper button (turns green when variant is uncached)
  btnCompressActive.addEventListener('click', async () => {
    if (!activeWallpaper) {
      showToast('Please select or play a wallpaper first to optimize it', 'info');
      return;
    }
    btnCompressActive.disabled = true;
    const origHtml = btnCompressActive.innerHTML;
    btnCompressActive.innerHTML = `${Icons.spinner} <span>Encoding Variant Q${currentQuality}...</span>`;

    try {
      const updated = await invoke('compress_wallpaper', {
        id: activeWallpaper.id,
        qualityLevel: currentQuality,
        targetFps: currentFps,
      });
      activeWallpaper = updated;
      const idx = wallpapers.findIndex((w) => w.id === updated.id);
      if (idx !== -1) wallpapers[idx] = updated;

      // Update button state (no longer uncached)
      checkVariantCacheState();
      renderLibrary();
      showToast(`Successfully cached Q${currentQuality} for "${activeWallpaper.title}"`, 'success');
      await refreshCacheStats();
    } catch (err) {
      showToast('Compression error: ' + err, 'error');
    } finally {
      btnCompressActive.disabled = false;
      btnCompressActive.innerHTML = origHtml;
      checkVariantCacheState();
    }
  });

  // Compress Entire Library button
  btnCompressAll.addEventListener('click', async () => {
    if (wallpapers.length === 0) {
      showToast('No wallpapers in library to compress', 'info');
      return;
    }

    btnCompressAll.disabled = true;
    const origHtml = btnCompressAll.innerHTML;
    btnCompressAll.innerHTML = `${Icons.spinner} <span>Encoding Library Variants...</span>`;

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
    checkVariantCacheState();
    renderLibrary();
    showToast(`Successfully cached variant Q${currentQuality} for ${count} wallpaper(s)`, 'success');
    await refreshCacheStats();
  });
}

// On quality slider change: instant switch if variant cached, else turn button green!
async function onQualitySliderChanged(val) {
  currentQuality = val;
  sliderQuality.value = val;
  settingQualitySlider.value = val;

  const profile = QUALITY_PROFILES[val] || QUALITY_PROFILES[5];
  labelQualityVal.textContent = profile.badge;
  settingQualityTitle.textContent = profile.title;
  settingQualityDesc.textContent = profile.desc;

  // 1. Immediately update UI optimize button highlights synchronously for 0ms latency
  checkVariantCacheState();

  // 2. Perform backend synchronization in background without stalling UI animations
  syncPerformanceSettings().catch((e) => console.error('Failed to sync performance settings:', e));

  // 3. Variant caching behavior for active wallpaper
  if (activeWallpaper) {
    const isCached = (activeWallpaper.cached_qualities || [5]).includes(val);
    if (isCached) {
      try {
        const updated = await invoke('switch_wallpaper_quality', {
          id: activeWallpaper.id,
          qualityLevel: val,
        });
        activeWallpaper = updated;
        const idx = wallpapers.findIndex((w) => w.id === updated.id);
        if (idx !== -1) wallpapers[idx] = updated;
        nowPlayingSub.textContent = `Playing on Desktop (${updated.format.toUpperCase()} • ${updated.resolution || 'HD'})`;
        checkVariantCacheState();
      } catch (e) {
        console.error('Failed to switch to cached variant:', e);
      }
    }
  }
}

// Updates the per-card optimize button states across the library grid
function updateLibraryOptimizeButtons() {
  wallpapers.forEach((wp) => {
    const isCachedForLevel = (wp.cached_qualities || [5]).includes(currentQuality);
    const optBtn = document.getElementById(`btn-opt-${wp.id}`);
    if (optBtn) {
      if (!isCachedForLevel) {
        optBtn.classList.add('btn-optimize-ready');
        optBtn.setAttribute('title', `Optimize / Cache Q${currentQuality}`);
      } else {
        optBtn.classList.remove('btn-optimize-ready');
        optBtn.setAttribute('title', `Variant Q${currentQuality} Cached`);
      }
    }
  });
}

// Makes Optimize / Compress buttons turn green if variant is NOT yet cached!
function checkVariantCacheState() {
  // 1. Active wallpaper optimize button
  if (!activeWallpaper) {
    btnCompressActive.classList.remove('btn-optimize-ready');
    const labelSpan = btnCompressActive.querySelector('span:last-child');
    if (labelSpan) {
      labelSpan.textContent = 'Optimize / Cache Current Variant';
    }
  } else {
    const cached = activeWallpaper.cached_qualities || [5];
    const isCached = cached.includes(currentQuality);

    if (!isCached) {
      btnCompressActive.classList.add('btn-optimize-ready');
      const labelSpan = btnCompressActive.querySelector('span:last-child');
      if (labelSpan) {
        labelSpan.textContent = `Optimize & Cache Q${currentQuality}`;
      }
    } else {
      btnCompressActive.classList.remove('btn-optimize-ready');
      const labelSpan = btnCompressActive.querySelector('span:last-child');
      if (labelSpan) {
        labelSpan.textContent = `Variant Q${currentQuality} Cached`;
      }
    }
  }

  // 2. Cache entire library button
  const uncachedCount = wallpapers.filter(
    (w) => !(w.cached_qualities || [5]).includes(currentQuality)
  ).length;
  if (btnCompressAll) {
    const labelSpanAll = btnCompressAll.querySelector('span:last-child');
    if (uncachedCount > 0) {
      btnCompressAll.classList.add('btn-optimize-ready');
      if (labelSpanAll) {
        labelSpanAll.textContent = `Cache All Variants (${uncachedCount} Uncached Q${currentQuality})`;
      }
    } else {
      btnCompressAll.classList.remove('btn-optimize-ready');
      if (labelSpanAll) {
        labelSpanAll.textContent = `All Library Variants Cached (Q${currentQuality})`;
      }
    }
  }

  // 3. Per-card optimize buttons in library
  updateLibraryOptimizeButtons();
}

async function loadPerformanceSettings() {
  try {
    const s = await invoke('get_performance_settings');
    currentQuality = s.quality_level || 5;
    currentFps = s.target_fps || 30;
    currentFillMode = s.fill_mode !== undefined ? s.fill_mode : 2;
    currentPanX = s.pan_x || 0.0;
    currentPanY = s.pan_y || 0.0;
    currentPausePolicy = s.pause_policy !== undefined ? s.pause_policy : 1;

    sliderQuality.value = currentQuality;
    settingQualitySlider.value = currentQuality;
    const profile = QUALITY_PROFILES[currentQuality] || QUALITY_PROFILES[5];
    labelQualityVal.textContent = profile.badge;
    settingQualityTitle.textContent = profile.title;
    settingQualityDesc.textContent = profile.desc;

    settingFpsSelect.value = currentFps.toString();
    settingPausePolicy.value = currentPausePolicy.toString();
    selectSizing.value = currentFillMode.toString();
    setPanSelectValue(selectPanX, currentPanX);
    setPanSelectValue(selectPanY, currentPanY);
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
      pauseOnFullscreen: currentPausePolicy === 1,
      pausePolicy: currentPausePolicy,
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
        showToast(`Imported ${imported.length} new wallpaper(s)`, 'success');
        await refreshCacheStats();
      }
    } catch (e) {
      showToast('Error importing video: ' + e, 'error');
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
          cached_qualities: [5],
        };
        installingItems.unshift(tempItem);
        addedTemp.push({ path: p, tempId: tempItem.id });
      }
      renderLibrary();

      for (const item of addedTemp) {
        try {
          const imported = await invoke('import_wallpaper', { sourcePath: item.path });
          installingItems = installingItems.filter((i) => i.id !== item.tempId);
          wallpapers.unshift(imported);

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

      setTimeout(async () => {
        renderLibrary();
        showToast(`Imported ${addedTemp.length} wallpaper(s)`, 'success');
        await refreshCacheStats();
      }, 600);
    }
  });

  dropzone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropzone.classList.add('dragover');
  });

  dropzone.addEventListener('dragleave', () => {
    dropzone.classList.remove('dragleave');
  });
}

// Window Controls & Exit Modal Logic
function setupWindowControls() {
  const getCloseBehavior = () => localStorage.getItem('close_behavior') || 'tray';

  const updateSettingDropdown = () => {
    if (settingCloseBehavior) {
      settingCloseBehavior.value = getCloseBehavior();
    }
  };

  updateSettingDropdown();


  // Settings dropdown change
  if (settingCloseBehavior) {
    settingCloseBehavior.addEventListener('change', (e) => {
      localStorage.setItem('close_behavior', e.target.value);
    });
  }

  // Reset prompt button to default (tray)
  if (btnResetClosePref) {
    btnResetClosePref.addEventListener('click', () => {
      localStorage.setItem('close_behavior', 'tray');
      updateSettingDropdown();
    });
  }

  const showCloseDialog = () => {
    if (chkRememberCloseAction) chkRememberCloseAction.checked = false;
    modalCloseControls.classList.remove('hidden');
  };

  const hideCloseDialog = () => {
    modalCloseControls.classList.add('hidden');
  };

  const executeCloseAction = async (action) => {
    if (chkRememberCloseAction && chkRememberCloseAction.checked) {
      localStorage.setItem('close_behavior', action);
      updateSettingDropdown();
    }
    hideCloseDialog();

    try {
      if (action === 'tray') {
        await invoke('hide_window');
      } else if (action === 'minimize') {
        await invoke('minimize_window');
      } else if (action === 'quit') {
        await invoke('safe_exit_app');
      }
    } catch (err) {
      console.error(`Failed to execute close action ${action}:`, err);
    }
  };

  const triggerCloseFlow = () => {
    const pref = getCloseBehavior();
    if (pref === 'tray') {
      invoke('hide_window').catch(console.error);
    } else if (pref === 'minimize') {
      invoke('minimize_window').catch(console.error);
    } else if (pref === 'quit') {
      invoke('safe_exit_app').catch(console.error);
    } else {
      showCloseDialog();
    }
  };

  // Intercept native OS window close requests emitted from Tauri backend
  listen('window-close-requested', () => {
    triggerCloseFlow();
  });

  // Dialog option buttons
  if (btnOptTray) {
    btnOptTray.addEventListener('click', () => executeCloseAction('tray'));
  }

  if (btnOptMinimize) {
    btnOptMinimize.addEventListener('click', () => executeCloseAction('minimize'));
  }

  if (btnOptQuit) {
    btnOptQuit.addEventListener('click', () => executeCloseAction('quit'));
  }

  // Dialog cancel/dismiss buttons
  if (btnCloseDialogCancel) {
    btnCloseDialogCancel.addEventListener('click', hideCloseDialog);
  }

  if (btnDialogCancelFooter) {
    btnDialogCancelFooter.addEventListener('click', hideCloseDialog);
  }

  modalCloseControls.addEventListener('click', (e) => {
    if (e.target === modalCloseControls) {
      hideCloseDialog();
    }
  });

  // Escape key closes modal
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !modalCloseControls.classList.contains('hidden')) {
      hideCloseDialog();
    }
  });

  // Custom Frameless Window Controls
  const btnWinMin = document.getElementById('btn-win-min');
  const btnWinMax = document.getElementById('btn-win-max');
  const btnWinClose = document.getElementById('btn-win-close');
  const headerEl = document.querySelector('.header');

  const updateMaximizeState = async () => {
    if (!btnWinMax) return;
    try {
      const isMaximized = await invoke('is_window_maximized');
      if (isMaximized) {
        btnWinMax.innerHTML = Icons.winRestore;
        btnWinMax.title = 'Restore';
        btnWinMax.setAttribute('aria-label', 'Restore');
        document.body.classList.add('maximized');
      } else {
        btnWinMax.innerHTML = Icons.winMax;
        btnWinMax.title = 'Maximize';
        btnWinMax.setAttribute('aria-label', 'Maximize');
        document.body.classList.remove('maximized');
      }
    } catch (err) {
      console.error('Failed to query window maximize state:', err);
    }
  };

  if (btnWinMin) {
    btnWinMin.addEventListener('click', () => {
      invoke('minimize_window').catch(console.error);
    });
  }

  if (btnWinMax) {
    btnWinMax.addEventListener('click', async () => {
      try {
        await invoke('toggle_maximize_window');
        setTimeout(updateMaximizeState, 50);
      } catch (err) {
        console.error('Failed to toggle maximize window:', err);
      }
    });
  }

  if (btnWinClose) {
    btnWinClose.addEventListener('click', () => {
      triggerCloseFlow();
    });
  }

  if (headerEl) {
    headerEl.addEventListener('dblclick', async (e) => {
      if (e.target.closest('button, input, select, textarea, .nav-tab, .window-controls')) return;
      try {
        await invoke('toggle_maximize_window');
        setTimeout(updateMaximizeState, 50);
      } catch (err) {
        console.error('Failed to toggle maximize on header dblclick:', err);
      }
    });
  }

  window.addEventListener('resize', () => {
    updateMaximizeState();
  });

  updateMaximizeState();
}

// In-App Deletion Modal Logic
function setupDeleteModal() {
  if (btnConfirmDelete) {
    btnConfirmDelete.addEventListener('click', async () => {
      if (!pendingDeleteId) return;
      const idToDelete = pendingDeleteId;
      hideDeleteConfirm();
      try {
        await invoke('delete_wallpaper', { id: idToDelete });
        showToast('Wallpaper deleted', 'info');
        await loadLibrary();
        await refreshCacheStats();
      } catch (e) {
        showToast(`Error deleting wallpaper: ${e}`, 'error');
      }
    });
  }

  if (btnCancelDelete) {
    btnCancelDelete.addEventListener('click', hideDeleteConfirm);
  }

  if (btnCancelDeleteX) {
    btnCancelDeleteX.addEventListener('click', hideDeleteConfirm);
  }

  if (modalDeleteConfirm) {
    modalDeleteConfirm.addEventListener('click', (e) => {
      if (e.target === modalDeleteConfirm) {
        hideDeleteConfirm();
      }
    });
  }
}

function showDeleteConfirm(id) {
  const wp = wallpapers.find((w) => w.id === id);
  if (!wp) return;
  pendingDeleteId = id;
  if (modalDeleteDesc) {
    modalDeleteDesc.textContent = `Are you sure you want to remove "${wp.title}"?`;
  }
  if (modalDeleteConfirm) {
    modalDeleteConfirm.classList.remove('hidden');
  }
}

function hideDeleteConfirm() {
  pendingDeleteId = null;
  if (modalDeleteConfirm) {
    modalDeleteConfirm.classList.add('hidden');
  }
}

// Global Keyboard Navigation
function setupKeyboardShortcuts() {
  window.addEventListener('keydown', (e) => {
    const isInputFocused = ['INPUT', 'SELECT', 'TEXTAREA'].includes(document.activeElement?.tagName);

    // Space: Toggle Play/Pause on active wallpaper
    if (e.code === 'Space' && !isInputFocused) {
      e.preventDefault();
      if (activeWallpaper && btnTogglePlay) {
        btnTogglePlay.click();
      }
      return;
    }

    // Ctrl+F: Focus Search
    if (e.ctrlKey && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      const libTabBtn = document.getElementById('tab-btn-library');
      if (libTabBtn && !libTabBtn.classList.contains('active')) {
        libTabBtn.click();
      }
      if (searchLibraryInput) {
        searchLibraryInput.focus();
        searchLibraryInput.select();
      }
      return;
    }

    // Ctrl+O or Ctrl+I: Import Video
    if (e.ctrlKey && (e.key.toLowerCase() === 'o' || e.key.toLowerCase() === 'i')) {
      e.preventDefault();
      if (btnImport) {
        btnImport.click();
      }
      return;
    }

    // Escape: Clear search if focused, or close active modals
    if (e.key === 'Escape') {
      if (document.activeElement === searchLibraryInput) {
        searchLibraryInput.value = '';
        searchQuery = '';
        if (btnClearSearch) btnClearSearch.classList.add('hidden');
        searchLibraryInput.blur();
        renderLibrary();
        return;
      }
      if (modalDeleteConfirm && !modalDeleteConfirm.classList.contains('hidden')) {
        hideDeleteConfirm();
        return;
      }
      if (modalCloseControls && !modalCloseControls.classList.contains('hidden')) {
        modalCloseControls.classList.add('hidden');
        return;
      }
    }
  });
}

// Quick-Jump from Player Bar to Library Card
function setupPlayerInfoJump() {
  if (playerInfoJump) {
    playerInfoJump.addEventListener('click', () => {
      if (!activeWallpaper) return;
      const libTabBtn = document.getElementById('tab-btn-library');
      if (libTabBtn && !libTabBtn.classList.contains('active')) {
        libTabBtn.click();
      }

      setTimeout(() => {
        const card = document.getElementById(`wp-card-${activeWallpaper.id}`);
        if (card) {
          card.scrollIntoView({ behavior: 'smooth', block: 'center' });
          card.classList.remove('highlight-pulse');
          void card.offsetWidth;
          card.classList.add('highlight-pulse');
          showToast(`Located active wallpaper: ${activeWallpaper.title}`, 'info');
        }
      }, 60);
    });
  }
}

// Storage & Cache Management
async function refreshCacheStats() {
  const cacheStatsText = document.getElementById('cache-stats-text');
  if (!cacheStatsText) return;
  try {
    const stats = await invoke('get_cache_stats');
    const mb = (stats.total_cache_bytes / (1024 * 1024)).toFixed(1);
    cacheStatsText.textContent = `Cached Variants: ${stats.cached_variants_count} file(s) (${mb} MB)`;
  } catch (e) {
    console.error('Failed to get cache stats:', e);
  }
}

function setupStorageControls() {
  const handleOpenStorage = async () => {
    try {
      await invoke('open_storage_dir');
    } catch (e) {
      showToast(`Failed to open folder: ${e}`, 'error');
    }
  };

  if (btnOpenFolder) {
    btnOpenFolder.addEventListener('click', handleOpenStorage);
  }

  if (btnOpenFolderSettings) {
    btnOpenFolderSettings.addEventListener('click', handleOpenStorage);
  }

  if (btnClearCache) {
    btnClearCache.addEventListener('click', async () => {
      try {
        const deleted = await invoke('clear_variant_cache');
        showToast(`Cleared ${deleted} variant files`, 'info');
        await refreshCacheStats();
        checkVariantCacheState();
        renderLibrary();
      } catch (e) {
        showToast(`Failed to clear cache: ${e}`, 'error');
      }
    });
  }
}

// System Autostart Controls
async function setupAutostartControls() {
  if (!chkAutostart) return;

  const updateAutostartUi = (enabled) => {
    chkAutostart.checked = enabled;
    if (labelAutostartStatus) {
      labelAutostartStatus.textContent = enabled ? 'Enabled' : 'Disabled';
      labelAutostartStatus.style.color = enabled ? '#10b981' : 'var(--text-secondary)';
    }
  };

  try {
    const isAutostart = await invoke('get_autostart_status');
    updateAutostartUi(isAutostart);
  } catch (err) {
    console.error('Failed to query autostart status:', err);
  }

  chkAutostart.addEventListener('change', async (e) => {
    const targetState = e.target.checked;
    try {
      const result = await invoke('set_autostart', { enable: targetState });
      updateAutostartUi(result);
      if (result) {
        showToast('Animated Linux added to system startup', 'success');
      } else {
        showToast('Animated Linux removed from system startup', 'info');
      }
    } catch (err) {
      showToast(`Failed to update startup configuration: ${err}`, 'error');
      try {
        const isAutostart = await invoke('get_autostart_status');
        updateAutostartUi(isAutostart);
      } catch (_) {}
    }
  });
}

// Start Application
init();
