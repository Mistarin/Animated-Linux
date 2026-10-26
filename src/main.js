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
let currentPausePolicy = 0; // 0: Never, 1: Fullscreen Game/App, 2: Maximized Window, 3: Any Window, 4: Battery
let currentPauseDetection = 'individual'; // 'individual' | 'main' | 'all'
let currentMainDisplayId = 0;
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
const playerTimeDisplay = document.getElementById('player-time-display');
const playerQualityPill = document.getElementById('player-quality-pill');
const btnSizingToggle = document.getElementById('btn-sizing-toggle');
const sizingPopoverMenu = document.getElementById('sizing-popover-menu');
const sizingPopoverContainer = document.getElementById('sizing-popover-container');

// Settings Elements
const settingQualitySlider = document.getElementById('setting-quality-slider');
const settingQualityTitle = document.getElementById('setting-quality-title');
const settingQualityDesc = document.getElementById('setting-quality-desc');
const settingFpsSelect = document.getElementById('setting-fps-select');
const settingPausePolicy = document.getElementById('setting-pause-policy');
const settingPauseScope = document.getElementById('setting-pause-scope');
const selectMainDisplayTop = document.getElementById('select-main-display-top');
const mainDisplayHint = document.getElementById('main-display-hint');
const hintMainDisplayName = document.getElementById('hint-main-display-name');
const linkConfigureDisplays = document.getElementById('link-configure-displays');
const descPauseDetection = document.getElementById('desc-pause-detection');
const btnResetDesktop = document.getElementById('btn-reset-desktop');
const btnCompressActive = document.getElementById('btn-compress-active');
const btnCompressAll = document.getElementById('btn-compress-all');
const cachingProgressContainer = document.getElementById('caching-progress-container');
const cachingProgressBar = document.getElementById('caching-progress-bar');
const cachingProgressCount = document.getElementById('caching-progress-count');
const cachingProgressDetail = document.getElementById('caching-progress-detail');
const cachingProgressTitle = document.getElementById('caching-progress-title');
const cachingProgressSpinner = document.getElementById('caching-progress-spinner');
const btnCancelCaching = document.getElementById('btn-cancel-caching');
const chkHardwareAccel = document.getElementById('chk-hardware-accel');
const chkInactiveBlack = document.getElementById('chk-inactive-black');
const chkRestartOnResume = document.getElementById('chk-restart-on-resume');
const settingBackend = document.getElementById('setting-backend');
const chkAutostart = document.getElementById('chk-autostart');
const chkStartMinimized = document.getElementById('chk-start-minimized');
const chkDebugWindow = document.getElementById('chk-debug-window');
const badgeDebugStatus = document.getElementById('badge-debug-status');
const labelDebugStatus = document.getElementById('label-debug-status');
const btnOpenDebugModal = document.getElementById('btn-open-debug-modal');
const modalDebugDiagnostics = document.getElementById('modal-debug-diagnostics');
const btnCloseDebugModal = document.getElementById('btn-close-debug-modal');
const btnCloseDebugModalX = document.getElementById('btn-close-debug-modal-x');
const btnRefreshDebugLogs = document.getElementById('btn-refresh-debug-logs');
const btnCopyDebugLogs = document.getElementById('btn-copy-debug-logs');
const debugDiagnosticsSummary = document.getElementById('debug-diagnostics-summary');
const debugLogsOutput = document.getElementById('debug-logs-output');

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

// Batch Import Modal & Button Elements
const modalBatchImport = document.getElementById('modal-batch-import');
const modalBatchTitle = document.getElementById('modal-batch-title');
const modalBatchStatus = document.getElementById('modal-batch-status');
const batchProgressLabel = document.getElementById('batch-progress-label');
const batchProgressPct = document.getElementById('batch-progress-pct');
const batchProgressFill = document.getElementById('batch-progress-fill');
const batchProgressCounts = document.getElementById('batch-progress-counts');
const batchSpinner = document.getElementById('batch-spinner');
const batchStatusText = document.getElementById('batch-status-text');
const btnBatchClose = document.getElementById('btn-batch-close');
const btnImportFolder = document.getElementById('btn-import-folder');

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

const DEFAULT_THUMB_SVG =
  'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180" viewBox="0 0 320 180" fill="%23131316"><rect width="320" height="180" fill="%23131316"/><rect x="100" y="50" width="120" height="80" rx="8" fill="%23222228" stroke="%2333333d" stroke-width="2"/><polygon points="152,78 152,102 174,90" fill="%2371717a"/></svg>';

// Helper to convert or extract thumbnail URL (prioritizes base64 data for WebKitGTK)
function getThumbnail(wp) {
  if (!wp) return DEFAULT_THUMB_SVG;
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
  return DEFAULT_THUMB_SVG;
}

// Populate Static Icons
function renderStaticIcons() {
  const setIcon = (id, iconHtml) => {
    const el = document.getElementById(id);
    if (el) el.innerHTML = iconHtml;
  };

  setIcon('icon-import-slot', Icons.plus);
  setIcon('icon-import-folder-slot', Icons.folderPlus);
  setIcon('modal-batch-icon', Icons.folderPlus);
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
  setIcon('caching-progress-spinner', Icons.spinner);
  setIcon('icon-col-engine', Icons.sliders);
  setIcon('icon-col-system', Icons.settings);

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
  setupAppMenuControls();
  setupBackendControls();
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

      if (targetTab === 'displays') {
        refreshDisplays();
      } else if (targetTab === 'settings') {
        refreshCacheStats();
      }
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
      localStorage.setItem('library_sort', currentSort);
      renderLibrary();
    });
  }
}

// Displays
function getMainDisplayName() {
  const found = displays.find((d) => d.id === currentMainDisplayId);
  return found ? found.name : `Display ${currentMainDisplayId + 1}`;
}

function updateMainDisplayHint() {
  if (!mainDisplayHint || !hintMainDisplayName) return;
  const isMain = settingPauseScope && settingPauseScope.value === 'main';
  if (isMain) {
    mainDisplayHint.style.display = 'flex';
    hintMainDisplayName.textContent = getMainDisplayName();
  } else {
    mainDisplayHint.style.display = 'none';
  }
}

async function refreshDisplays() {
  try {
    displays = await invoke('get_monitors');
    displayCount.textContent = displays.length;

    // Detect if default main display should be set from system primary
    const primary = displays.find((d) => d.is_main);
    if (primary && localStorage.getItem('main_display_id') === null && currentMainDisplayId === 0) {
      currentMainDisplayId = primary.id;
    }

    renderDisplays();
    updateMainDisplayHint();
  } catch (e) {
    console.error('Failed to get monitors:', e);
  }
}

function renderDisplays() {
  if (!displaysList) return;

  if (selectMainDisplayTop) {
    selectMainDisplayTop.innerHTML = displays
      .map(
        (d) =>
          `<option value="${d.id}" ${d.id === currentMainDisplayId ? 'selected' : ''}>${d.name} (${d.resolution}) ${d.id === currentMainDisplayId ? '(Main)' : ''}</option>`
      )
      .join('');
  }

  displaysList.innerHTML = displays
    .map((d) => {
      const isMain = d.id === currentMainDisplayId;
      return `
    <div class="display-card ${isMain ? 'display-card-main' : ''}" style="background: var(--bg-card); border: 1px solid ${isMain ? 'rgba(59, 130, 246, 0.45)' : 'var(--border-card)'}; border-radius: var(--radius-md); padding: 18px 20px; display: flex; justify-content: space-between; align-items: center; transition: var(--transition-fast);">
      <div style="display: flex; align-items: center; gap: 16px;">
        <div style="color: ${isMain ? '#60a5fa' : 'var(--text-muted)'}; font-size: 1.25rem;">${Icons.displays}</div>
        <div>
          <div style="display: flex; align-items: center; gap: 8px;">
            <span style="font-weight: 500; font-size: 0.95rem; color: var(--text-main);">${d.name}</span>
            ${isMain ? `<span class="badge-main-display">Main Display Source</span>` : ''}
          </div>
          <div style="color: var(--text-muted); font-size: 0.78rem; margin-top: 3px;">
            Resolution: ${d.resolution} • ID: ${d.id} ${isMain ? '• Active window detection source' : ''}
          </div>
        </div>
      </div>
      <div style="display: flex; align-items: center; gap: 10px;">
        ${!isMain ? `
          <button class="btn btn-secondary btn-set-main" onclick="window.setAsMainDisplay(${d.id})" title="Set this screen as the Main Display Source for active window tracking">
            Set as Main Source
          </button>
        ` : `
          <button class="btn btn-secondary btn-set-main" disabled style="opacity: 0.65; cursor: default;">
            Main Source
          </button>
        `}
        <button class="btn btn-primary" onclick="window.applyToDisplay(${d.id})">
          Apply Wallpaper
        </button>
      </div>
    </div>
  `;
    })
    .join('');
}

window.setAsMainDisplay = async (screenId) => {
  currentMainDisplayId = screenId;
  const target = displays.find((d) => d.id === screenId);
  const targetName = target ? target.name : `Display ${screenId}`;

  try {
    await invoke('set_main_display', { displayId: screenId });
  } catch (err) {
    console.warn('Backend set_main_display call:', err);
  }
  localStorage.setItem('main_display_id', screenId.toString());
  renderDisplays();
  updateMainDisplayHint();
  showToast(`"${targetName}" set as Main Display Source`, 'success');
};

window.applyToDisplay = async (screenId) => {
  if (!activeWallpaper) {
    showToast('Please select a wallpaper first', 'info');
    return;
  }
  await applyWallpaper(activeWallpaper, screenId);
  showToast(`Applied "${activeWallpaper.title}" to display ${screenId}`, 'success');
};

// Active Wallpaper Duration and Quality Badge Helpers
function formatDuration(secs) {
  if (!secs || isNaN(secs) || secs <= 0) return '0:00';
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

function updateActivePlayerBadge(wp) {
  if (playerTimeDisplay) {
    playerTimeDisplay.textContent = wp && wp.duration_secs ? formatDuration(wp.duration_secs) : '0:00';
  }
  if (playerQualityPill) {
    const profile = QUALITY_PROFILES[currentQuality] || QUALITY_PROFILES[5];
    playerQualityPill.textContent = profile.badge;
  }
}

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
        const s = await invoke('get_performance_settings');
        const isPaused = s ? s.is_paused : false;
        isPlaying = !isPaused;
        nowPlayingSub.textContent = isPlaying
          ? `Playing on Desktop (${active.format.toUpperCase()} • ${active.resolution || 'HD'})`
          : `Paused (${active.format.toUpperCase()} • ${active.resolution || 'HD'})`;
        btnTogglePlay.innerHTML = isPlaying ? Icons.pause : Icons.play;
        updateActivePlayerBadge(active);
      } else {
        updateActivePlayerBadge(null);
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
          <div class="card-thumbnail-wrapper is-installing" id="thumb-wrapper-${wp.id}">
            <img src="${thumbSrc}" class="card-thumb" id="thumb-img-${wp.id}" alt="${wp.title}" onerror="this.onerror=null; this.src='${DEFAULT_THUMB_SVG}'" />
            <span class="badge-installing">Importing</span>
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
          <img src="${thumbSrc}" class="card-thumb" id="thumb-img-${wp.id}" alt="${wp.title}" onerror="this.onerror=null; this.src='${DEFAULT_THUMB_SVG}'" />
          <video class="card-video-preview" 
                 id="video-preview-${wp.id}" 
                 data-src="${convertFileSrc(wp.file_path)}" 
                 muted 
                 loop 
                 playsinline 
                 preload="none" 
                 onerror="this.style.display='none'"></video>
          ${isActive ? `<span class="badge-active">${isPlaying ? 'Playing' : 'Paused'}</span>` : ''}
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

  // Interactive hover video previews
  libraryGrid.querySelectorAll('.card').forEach((card) => {
    const video = card.querySelector('.card-video-preview');
    if (!video) return;

    card.addEventListener('mouseenter', () => {
      if (!video.src && video.dataset.src) {
        video.src = video.dataset.src;
        video.load();
      }
      const playPromise = video.play();
      if (playPromise && typeof playPromise.catch === 'function') {
        playPromise.catch(() => {});
      }
    });

    card.addEventListener('mouseleave', () => {
      if (video.src) {
        video.pause();
        video.currentTime = 0;
      }
    });
  });
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
    let targetWp = wp;
    if (currentQuality < 5 && (wp.cached_qualities || []).includes(currentQuality)) {
      try {
        const switched = await invoke('switch_wallpaper_quality', {
          id: wp.id,
          qualityLevel: currentQuality,
        });
        if (switched) {
          targetWp = switched;
          const idx = wallpapers.findIndex((w) => w.id === switched.id);
          if (idx !== -1) wallpapers[idx] = switched;
        }
      } catch (_) {}
    }

    await invoke('apply_wallpaper', {
      wallpaper: targetWp,
      screen: screenId !== null ? screenId : null,
    });
    activeWallpaper = targetWp;
    isPlaying = true;

    // Update bottom player
    nowPlayingThumb.src = getThumbnail(targetWp);
    nowPlayingTitle.textContent = targetWp.title;
    nowPlayingSub.textContent = `Playing on Desktop (${targetWp.format.toUpperCase()} • ${targetWp.resolution || 'HD'})`;
    btnTogglePlay.innerHTML = Icons.pause;
    updateActivePlayerBadge(targetWp);

    checkVariantCacheState();
    renderLibrary();
  } catch (e) {
    showToast('Failed to set wallpaper: ' + e, 'error');
  }
}

// Media Controls
function setupMediaControls() {
  btnTogglePlay.addEventListener('click', async () => {
    if (!activeWallpaper) {
      if (libraryItems && libraryItems.length > 0) {
        await applyWallpaper(libraryItems[0]);
      }
      return;
    }
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

  const modalStopConfirm = document.getElementById('modal-stop-confirm');
  const btnCancelStop = document.getElementById('btn-cancel-stop');
  const btnCancelStopX = document.getElementById('btn-cancel-stop-x');
  const btnConfirmStop = document.getElementById('btn-confirm-stop');

  const hideStopDialog = () => {
    if (modalStopConfirm) modalStopConfirm.classList.add('hidden');
  };

  if (btnCancelStop) btnCancelStop.addEventListener('click', hideStopDialog);
  if (btnCancelStopX) btnCancelStopX.addEventListener('click', hideStopDialog);
  if (modalStopConfirm) {
    modalStopConfirm.addEventListener('click', (e) => {
      if (e.target === modalStopConfirm) hideStopDialog();
    });
  }

  if (btnConfirmStop) {
    btnConfirmStop.addEventListener('click', async () => {
      hideStopDialog();
      try {
        await invoke('stop_wallpaper');
        activeWallpaper = null;
        isPlaying = false;
        nowPlayingTitle.textContent = 'No Wallpaper Active';
        nowPlayingSub.textContent = 'Desktop restored to default static wallpaper';
        btnTogglePlay.innerHTML = Icons.play;
        updateActivePlayerBadge(null);
        checkVariantCacheState();
        renderLibrary();
        showToast('Wallpaper stopped; desktop safely restored', 'info');
      } catch (e) {
        console.error(e);
        showToast('Failed to stop wallpaper: ' + e, 'error');
      }
    });
  }

  btnStopWp.addEventListener('click', () => {
    if (modalStopConfirm) {
      modalStopConfirm.classList.remove('hidden');
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
      showToast(`Playback speed set to ${currentSpeed}x`, 'info');
    } catch (err) {
      console.error(err);
    }
  });

  // Bottom Media Bar Quality Slider (1 to 5)
  sliderQuality.addEventListener('input', async (e) => {
    const val = parseInt(e.target.value);
    await onQualitySliderChanged(val);
  });
  sliderQuality.addEventListener('change', (e) => {
    const val = parseInt(e.target.value);
    const profile = QUALITY_PROFILES[val] || QUALITY_PROFILES[5];
    showToast(`Quality preference saved: ${profile.badge} (${profile.title})`, 'info');
  });

  // Video Adjustments Sizing Popover Menu Toggle & Outside-Click Listener
  if (btnSizingToggle && sizingPopoverMenu) {
    btnSizingToggle.addEventListener('click', (e) => {
      e.stopPropagation();
      const isHidden = sizingPopoverMenu.classList.toggle('hidden');
      btnSizingToggle.classList.toggle('active', !isHidden);
    });

    document.addEventListener('click', (e) => {
      if (sizingPopoverContainer && !sizingPopoverContainer.contains(e.target)) {
        sizingPopoverMenu.classList.add('hidden');
        btnSizingToggle.classList.remove('active');
      }
    });
  }
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

// Helper to set or add formatted speed value (0.5, 1.0, 1.5, etc.) in select dropdown
function setSpeedSelectValue(selectEl, speedVal) {
  if (!selectEl) return;
  const str = speedVal.toString();
  const existing = Array.from(selectEl.options).find(
    (opt) => Math.abs(parseFloat(opt.value) - speedVal) < 0.001
  );
  if (existing) {
    selectEl.value = existing.value;
  } else {
    const newOpt = new Option(`${speedVal}x`, str, true, true);
    selectEl.add(newOpt);
    selectEl.value = str;
  }
}

// Geometry / Sizing and Pan Controls
function setupGeometryControls() {
  selectSizing.addEventListener('change', async (e) => {
    currentFillMode = parseInt(e.target.value);
    await syncGeometry();
    const modes = ['Stretch', 'Fit Aspect', 'Fill / Zoom', 'Center'];
    showToast(`Display sizing set to ${modes[currentFillMode] || 'Custom'}`, 'info');
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
    showToast('Position offset reset to center (0, 0)', 'info');
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
  settingQualitySlider.addEventListener('change', (e) => {
    const val = parseInt(e.target.value);
    const profile = QUALITY_PROFILES[val] || QUALITY_PROFILES[5];
    showToast(`Quality preference saved: ${profile.badge} (${profile.title})`, 'info');
  });

  settingFpsSelect.addEventListener('change', async (e) => {
    currentFps = parseInt(e.target.value);
    await syncPerformanceSettings();
    showToast(`Target rendering FPS set to ${currentFps === 0 ? 'Source / Native' : currentFps + ' FPS'}`, 'info');
  });

  settingPausePolicy.addEventListener('change', async (e) => {
    currentPausePolicy = parseInt(e.target.value);
    await syncPerformanceSettings();
    showToast('Pausing policy preference saved', 'info');
  });

  chkHardwareAccel.addEventListener('change', async () => {
    await syncPerformanceSettings();
    showToast(`Hardware acceleration ${chkHardwareAccel.checked ? 'enabled' : 'disabled'}`, 'info');
  });

  if (chkInactiveBlack) {
    chkInactiveBlack.addEventListener('change', async () => {
      await syncPerformanceSettings();
      showToast(chkInactiveBlack.checked ? 'Inactive screen: Black screen mode enabled' : 'Inactive screen: Static image mode enabled', 'info');
    });
  }

  if (settingPauseScope) {
    settingPauseScope.addEventListener('change', async () => {
      currentPauseDetection = settingPauseScope.value;
      updateMainDisplayHint();
      await syncPerformanceSettings();
      const label =
        currentPauseDetection === 'main'
          ? `Main Display (${getMainDisplayName()})`
          : currentPauseDetection === 'individual'
          ? 'Individual Screens'
          : 'All Screens (Global)';
      showToast(`Active window detection set to: ${label}`, 'info');
    });
  }

  if (selectMainDisplayTop) {
    selectMainDisplayTop.addEventListener('change', async (e) => {
      await window.setAsMainDisplay(parseInt(e.target.value, 10));
    });
  }

  if (linkConfigureDisplays) {
    linkConfigureDisplays.addEventListener('click', (e) => {
      e.preventDefault();
      const tab = document.getElementById('tab-btn-displays');
      if (tab) tab.click();
    });
  }

  if (btnResetDesktop) {
    btnResetDesktop.addEventListener('click', async () => {
      try {
        await invoke('reset_desktop_wallpaper');
        activeWallpaper = null;
        isPlaying = false;
        nowPlayingTitle.textContent = 'No Wallpaper Active';
        nowPlayingSub.textContent = 'Desktop restored to default static wallpaper';
        btnTogglePlay.innerHTML = Icons.play;
        updateActivePlayerBadge(null);
        checkVariantCacheState();
        renderLibrary();
        showToast('Desktop successfully reset to default static wallpaper', 'success');
      } catch (err) {
        console.error('Failed to reset desktop:', err);
        showToast('Failed to reset desktop: ' + err, 'error');
      }
    });
  }

  // Helper to update caching progress UI
  function updateCachingProgressUI(payload) {
    if (!payload) return;
    if (cachingProgressContainer) cachingProgressContainer.style.display = 'block';
    if (cachingProgressBar) cachingProgressBar.style.width = `${payload.percent}%`;
    if (cachingProgressCount) cachingProgressCount.textContent = `${payload.current} / ${payload.total}`;
    if (cachingProgressDetail) {
      if (payload.status) {
        cachingProgressDetail.textContent = payload.status;
      } else if (payload.title) {
        cachingProgressDetail.textContent = `Encoding "${payload.title}" (Q${currentQuality} at ${payload.percent}%)...`;
      }
    }
  }

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

  // Compress Entire Library button (Asynchronous Bounded Multi-Threaded Queue with Dual-Channel Progress)
  btnCompressAll.addEventListener('click', async () => {
    if (wallpapers.length === 0) {
      showToast('No wallpapers in library to compress', 'info');
      return;
    }

    if (currentQuality === 5) {
      showToast('Native Q5 represents uncompressed source files', 'info');
      return;
    }

    const uncachedCount = wallpapers.filter(
      (w) => !(w.cached_qualities || [5]).includes(currentQuality)
    ).length;

    if (uncachedCount === 0) {
      showToast(`All library items already have Q${currentQuality} cached`, 'info');
      return;
    }

    btnCompressAll.disabled = true;
    const origHtml = btnCompressAll.innerHTML;
    btnCompressAll.innerHTML = `${Icons.spinner} <span>Encoding Variants (Parallel)...</span>`;

    if (cachingProgressContainer) {
      cachingProgressContainer.style.display = 'block';
      if (cachingProgressBar) cachingProgressBar.style.width = '0%';
      if (cachingProgressCount) cachingProgressCount.textContent = `0 / ${uncachedCount}`;
      if (cachingProgressTitle) cachingProgressTitle.textContent = `Caching Library Variants (Q${currentQuality})...`;
      if (cachingProgressDetail) cachingProgressDetail.textContent = 'Dispatching concurrent transcoding workers...';
      if (btnCancelCaching) btnCancelCaching.disabled = false;
    }

    try {
      const res = await invoke('compress_all_wallpapers', {
        qualityLevel: currentQuality,
        targetFps: currentFps,
      });

      // Synchronize library items
      const refreshed = await invoke('get_wallpapers');
      wallpapers = refreshed;
      if (activeWallpaper) {
        const foundActive = wallpapers.find((w) => w.id === activeWallpaper.id);
        if (foundActive) activeWallpaper = foundActive;
      }

      checkVariantCacheState();
      renderLibrary();
      await refreshCacheStats();

      if (res && res.cancelled) {
        showToast(`Batch caching cancelled (${res.success_count} finished)`, 'info');
      } else {
        showToast(`Successfully cached Q${currentQuality} for ${res.success_count} wallpaper(s)`, 'success');
      }
    } catch (err) {
      showToast('Batch caching error: ' + err, 'error');
    } finally {
      if (cachingProgressBar) cachingProgressBar.style.width = '100%';
      if (cachingProgressCount) cachingProgressCount.textContent = `${uncachedCount} / ${uncachedCount}`;
      if (cachingProgressDetail) cachingProgressDetail.textContent = 'Batch optimization complete.';
      btnCompressAll.disabled = false;
      btnCompressAll.innerHTML = origHtml;
      checkVariantCacheState();
      setTimeout(() => {
        if (cachingProgressContainer) {
          cachingProgressContainer.style.display = 'none';
        }
      }, 2500);
    }
  });

  if (btnCancelCaching) {
    btnCancelCaching.addEventListener('click', async () => {
      try {
        await invoke('cancel_batch_compression');
        if (cachingProgressDetail) {
          cachingProgressDetail.textContent = 'Cancellation requested, draining workers...';
        }
        btnCancelCaching.disabled = true;
      } catch (err) {
        console.error('Failed to cancel batch compression:', err);
      }
    });
  }

  // Live streaming events for variant caching progress
  listen('caching-progress', (event) => {
    updateCachingProgressUI(event.payload);
  });

  listen('wallpaper-cached', (event) => {
    const payload = event.payload;
    if (!payload) return;
    const wp = wallpapers.find((w) => w.id === payload.id);
    if (wp) {
      if (!wp.cached_qualities) wp.cached_qualities = [5];
      if (!wp.cached_qualities.includes(payload.quality_level)) {
        wp.cached_qualities.push(payload.quality_level);
        wp.cached_qualities.sort();
      }
    }
    checkVariantCacheState();
  });

  // System Tray Synchronization Events
  listen('wallpaper-playback-changed', (event) => {
    const payload = event.payload;
    if (!payload) return;
    isPlaying = !payload.is_paused;
    btnTogglePlay.innerHTML = isPlaying ? Icons.pause : Icons.play;
    if (activeWallpaper) {
      updateActivePlayerBadge(activeWallpaper);
    }
    renderLibrary();
  });

  listen('wallpaper-stopped', () => {
    activeWallpaper = null;
    isPlaying = false;
    nowPlayingTitle.textContent = 'No Wallpaper Active';
    nowPlayingSub.textContent = 'Desktop restored to default static wallpaper';
    btnTogglePlay.innerHTML = Icons.play;
    updateActivePlayerBadge(null);
    checkVariantCacheState();
    renderLibrary();
  });

  if (chkDebugWindow) {
    chkDebugWindow.addEventListener('change', async (e) => {
      const enabled = e.target.checked;
      updateDebugBadge(enabled);
      try {
        await invoke('set_debug_enabled', { enabled });
        showToast(
          enabled
            ? 'Desktop debug overlay window enabled'
            : 'Desktop debug overlay window disabled',
          'info'
        );
      } catch (err) {
        console.error('Failed to update debug window setting:', err);
        showToast('Failed to toggle debug overlay: ' + err, 'error');
      }
    });
  }

  const hideDebugModal = () => {
    if (modalDebugDiagnostics) modalDebugDiagnostics.classList.add('hidden');
  };

  const loadDebugDiagnostics = async () => {
    if (!debugDiagnosticsSummary || !debugLogsOutput) return;
    try {
      debugLogsOutput.textContent = 'Fetching engine logs...';
      const [backend, logs] = await Promise.all([
        invoke('get_wallpaper_backend').catch(() => 'unknown'),
        invoke('get_engine_logs').catch((e) => 'Failed to query logs: ' + e),
      ]);

      const policyNames = {
        0: 'Never Pause (Always Play)',
        1: 'Fullscreen Game / App',
        2: 'Maximized Window Active',
        3: 'Any Window Active',
        4: 'Battery Power',
      };

      const items = [
        { label: 'Active Backend', value: backend === 'kde' ? 'KDE Plasma 6 (Reborn)' : backend },
        { label: 'Playback Status', value: isPlaying ? 'Playing' : 'Paused / Idle' },
        { label: 'Active Wallpaper', value: activeWallpaper ? activeWallpaper.title : 'None' },
        { label: 'Pausing Trigger', value: policyNames[currentPausePolicy] || currentPausePolicy },
        { label: 'Window Detection Scope', value: currentPauseDetection },
        { label: 'Desktop Overlay', value: chkDebugWindow && chkDebugWindow.checked ? 'Visible' : 'Hidden' },
        { label: 'Hardware Accel', value: chkHardwareAccel && chkHardwareAccel.checked ? 'Enabled (VA-API)' : 'Software' },
        { label: 'Target FPS', value: currentFps ? `${currentFps} FPS` : 'Native Rate' },
      ];

      debugDiagnosticsSummary.innerHTML = items
        .map(
          (it) => `
        <div style="background: var(--bg-surface); padding: 8px 10px; border-radius: var(--radius-sm); border: 1px solid var(--border-subtle);">
          <div style="font-size: 0.72rem; color: var(--text-muted);">${it.label}</div>
          <div style="font-size: 0.82rem; font-weight: 500; color: var(--text-main); margin-top: 2px;">${it.value}</div>
        </div>
      `
        )
        .join('');

      debugLogsOutput.textContent = logs || 'No recent engine logs found.';
      debugLogsOutput.scrollTop = debugLogsOutput.scrollHeight;
    } catch (err) {
      debugLogsOutput.textContent = 'Error loading diagnostics: ' + err;
    }
  };

  if (btnOpenDebugModal) {
    btnOpenDebugModal.addEventListener('click', () => {
      if (modalDebugDiagnostics) {
        modalDebugDiagnostics.classList.remove('hidden');
        loadDebugDiagnostics();
      }
    });
  }

  if (btnCloseDebugModal) btnCloseDebugModal.addEventListener('click', hideDebugModal);
  if (btnCloseDebugModalX) btnCloseDebugModalX.addEventListener('click', hideDebugModal);
  if (modalDebugDiagnostics) {
    modalDebugDiagnostics.addEventListener('click', (e) => {
      if (e.target === modalDebugDiagnostics) hideDebugModal();
    });
  }

  if (btnRefreshDebugLogs) {
    btnRefreshDebugLogs.addEventListener('click', () => {
      loadDebugDiagnostics();
    });
  }

  if (btnCopyDebugLogs) {
    btnCopyDebugLogs.addEventListener('click', async () => {
      if (debugLogsOutput) {
        try {
          await navigator.clipboard.writeText(debugLogsOutput.textContent);
          showToast('Diagnostics logs copied to clipboard', 'success');
        } catch (_) {
          showToast('Failed to copy to clipboard', 'error');
        }
      }
    });
  }
}

// On quality slider change: instant switch if variant cached, else turn button green!
async function onQualitySliderChanged(val) {
  currentQuality = val;
  sliderQuality.value = val;
  settingQualitySlider.value = val;

  const profile = QUALITY_PROFILES[val] || QUALITY_PROFILES[5];
  labelQualityVal.textContent = profile.badge;
  if (playerQualityPill) playerQualityPill.textContent = profile.badge;
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
    currentFps = s.target_fps !== undefined ? s.target_fps : 30;
    currentFillMode = s.fill_mode !== undefined ? s.fill_mode : 2;
    currentPanX = s.pan_x !== undefined ? s.pan_x : 0.0;
    currentPanY = s.pan_y !== undefined ? s.pan_y : 0.0;
    currentPausePolicy = s.pause_policy !== undefined ? s.pause_policy : 0;
    currentSpeed = s.speed !== undefined ? s.speed : 1.0;

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
    setSpeedSelectValue(selectSpeed, currentSpeed);
    chkHardwareAccel.checked = s.hardware_accel;
    if (chkInactiveBlack) {
      chkInactiveBlack.checked = s.inactive_black_screen || false;
    }
    if (chkRestartOnResume) {
      chkRestartOnResume.checked = s.restart_on_resume || false;
    }
    if (chkStartMinimized && s.start_minimized !== undefined) {
      chkStartMinimized.checked = s.start_minimized;
    }
    if (chkDebugWindow && s.debug_enabled !== undefined) {
      chkDebugWindow.checked = s.debug_enabled;
      updateDebugBadge(s.debug_enabled);
    }
    if (s.main_display_id !== undefined) {
      currentMainDisplayId = s.main_display_id;
    } else {
      const savedMain = localStorage.getItem('main_display_id');
      if (savedMain !== null) {
        currentMainDisplayId = parseInt(savedMain, 10);
      }
    }

    if (s.pause_detection_mode) {
      currentPauseDetection = s.pause_detection_mode;
    } else if (s.pause_per_monitor !== undefined) {
      currentPauseDetection = s.pause_per_monitor ? 'individual' : 'all';
    }

    if (settingPauseScope) {
      settingPauseScope.value = currentPauseDetection;
      updateMainDisplayHint();
    }

    // Restore saved library sort preference
    const savedSort = localStorage.getItem('library_sort') || 'recent';
    currentSort = savedSort;
    if (selectLibrarySort) {
      selectLibrarySort.value = savedSort;
    }
  } catch (e) {
    console.error('Failed to load performance settings:', e);
  }
}

function updateDebugBadge(enabled) {
  if (!badgeDebugStatus || !labelDebugStatus) return;
  if (enabled) {
    badgeDebugStatus.className = 'settings-badge settings-badge-active';
    labelDebugStatus.textContent = 'Active on Desktop';
  } else {
    badgeDebugStatus.className = 'settings-badge settings-badge-inactive';
    labelDebugStatus.textContent = 'Disabled';
  }
}

async function syncPerformanceSettings() {
  try {
    const pauseMode = settingPauseScope ? settingPauseScope.value : currentPauseDetection;
    await invoke('update_performance_settings', {
      qualityLevel: currentQuality,
      targetFps: currentFps,
      pauseOnFullscreen: currentPausePolicy !== 0,
      pausePolicy: currentPausePolicy,
      hardwareAccel: chkHardwareAccel.checked,
      inactiveBlackScreen: chkInactiveBlack ? chkInactiveBlack.checked : false,
      restartOnResume: chkRestartOnResume ? chkRestartOnResume.checked : false,
      pausePerMonitor: pauseMode === 'individual',
      pauseDetectionMode: pauseMode,
      mainDisplayId: currentMainDisplayId,
      debugEnabled: chkDebugWindow ? chkDebugWindow.checked : false,
    });
  } catch (err) {
    console.error('Failed to sync performance settings:', err);
  }
}

// Import & Drag Drop with Batch Processing, Recursive Folder Scan, and Slick Modal/Inline Progress
function setupImportAndDrop() {
  // Listen for background batch import progress events
  listen('batch-import-progress', (event) => {
    const payload = event.payload;
    if (!payload) return;
    if (batchProgressFill) batchProgressFill.style.width = `${payload.percent}%`;
    if (batchProgressPct) batchProgressPct.textContent = `${payload.percent}%`;
    if (batchProgressLabel) batchProgressLabel.textContent = payload.filename || 'Processing...';
    if (batchProgressCounts) batchProgressCounts.textContent = `${payload.current} / ${payload.total}`;
    if (batchStatusText) batchStatusText.textContent = payload.status || 'Importing';
  });

  // Handle single video import with sleek blurred preview card
  async function handleImportSingleFilePath(p) {
    const filename = p.split('/').pop().replace(/\.[^/.]+$/, '');
    const tempItem = {
      id: 'temp_' + Date.now() + '_' + Math.random().toString(36).slice(2, 6),
      title: filename,
      thumbnail_data: DEFAULT_THUMB_SVG,
      format: p.split('.').pop() || 'mp4',
      duration_secs: 0,
      resolution: 'Importing...',
      isInstalling: true,
      cached_qualities: [5],
    };
    installingItems.unshift(tempItem);
    renderLibrary();

    // Fast blurred preview frame extraction
    invoke('get_quick_preview', { sourcePath: p })
      .then((previewData) => {
        if (previewData) {
          tempItem.thumbnail_data = previewData;
          const thumbEl = document.getElementById(`thumb-img-${tempItem.id}`);
          if (thumbEl) thumbEl.src = previewData;
        }
      })
      .catch(() => {});

    try {
      const imported = await invoke('import_wallpaper', { sourcePath: p });
      installingItems = installingItems.filter((i) => i.id !== tempItem.id);
      wallpapers.unshift(imported);

      const cardEl = document.getElementById(`wp-card-${tempItem.id}`);
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

      setTimeout(async () => {
        renderLibrary();
        showToast('Wallpaper imported successfully', 'success');
        await refreshCacheStats();
      }, 300);
    } catch (err) {
      console.error('Failed to import ' + p, err);
      showToast('Failed to import: ' + err, 'error');
      installingItems = installingItems.filter((i) => i.id !== tempItem.id);
      renderLibrary();
    }
  }

  // Unified batch importer supporting multiple files, folders, and recursive directory scans
  async function runBatchImport(sources) {
    if (!sources || sources.length === 0) return;

    // Scan sources to resolve all video files (including subdirectories)
    let resolvedFiles = [];
    try {
      resolvedFiles = await invoke('scan_video_sources', { sources });
    } catch (e) {
      console.error('Scan error:', e);
      resolvedFiles = sources;
    }

    if (!resolvedFiles || resolvedFiles.length === 0) {
      showToast('No supported video files found in selection', 'info');
      return;
    }

    // Single video file -> use fast inline loading card
    if (resolvedFiles.length === 1) {
      await handleImportSingleFilePath(resolvedFiles[0]);
      return;
    }

    // Multiple files -> open batch import progress modal
    if (modalBatchImport) modalBatchImport.classList.remove('hidden');
    if (btnBatchClose) btnBatchClose.classList.add('hidden');
    if (batchSpinner) batchSpinner.classList.remove('hidden');
    if (batchProgressFill) batchProgressFill.style.width = '0%';
    if (batchProgressPct) batchProgressPct.textContent = '0%';
    if (batchProgressLabel) batchProgressLabel.textContent = `Found ${resolvedFiles.length} videos. Initializing...`;
    if (batchProgressCounts) batchProgressCounts.textContent = `0 / ${resolvedFiles.length}`;
    if (modalBatchStatus) modalBatchStatus.textContent = `Importing ${resolvedFiles.length} videos...`;
    if (batchStatusText) batchStatusText.textContent = 'Starting';

    try {
      const result = await invoke('batch_import_wallpapers', { sourcePaths: resolvedFiles });
      if (result && result.imported && result.imported.length > 0) {
        wallpapers = [...result.imported, ...wallpapers];
        renderLibrary();
        showToast(`Successfully imported ${result.imported.length} wallpaper(s)`, 'success');
        await refreshCacheStats();
      }

      if (batchProgressFill) batchProgressFill.style.width = '100%';
      if (batchProgressPct) batchProgressPct.textContent = '100%';
      if (batchSpinner) batchSpinner.classList.add('hidden');
      if (modalBatchStatus) modalBatchStatus.textContent = 'Import complete';
      if (batchStatusText) batchStatusText.textContent = 'Finished';
      if (batchProgressLabel) {
        batchProgressLabel.textContent = result.failed?.length > 0
          ? `Imported ${result.imported.length} videos (${result.failed.length} failed)`
          : `Imported all ${result.imported.length} videos successfully`;
      }

      setTimeout(() => {
        if (modalBatchImport) modalBatchImport.classList.add('hidden');
      }, 1000);
    } catch (err) {
      console.error('Batch import failed:', err);
      showToast('Batch import failed: ' + err, 'error');
      if (batchStatusText) batchStatusText.textContent = 'Failed';
      if (batchSpinner) batchSpinner.classList.add('hidden');
      if (btnBatchClose) btnBatchClose.classList.remove('hidden');
    }
  }

  async function triggerNativeFilePicker() {
    try {
      btnImport.disabled = true;
      btnImport.innerHTML = `${Icons.spinner} <span>Selecting...</span>`;
      const paths = await invoke('select_video_files');
      if (paths && paths.length > 0) {
        await runBatchImport(paths);
      }
    } catch (e) {
      showToast('Error importing video: ' + e, 'error');
    } finally {
      btnImport.disabled = false;
      btnImport.innerHTML = `${Icons.plus} <span>Import Video</span>`;
    }
  }

  async function triggerNativeFolderPicker() {
    try {
      if (btnImportFolder) {
        btnImportFolder.disabled = true;
        btnImportFolder.innerHTML = `${Icons.spinner} <span>Selecting...</span>`;
      }
      const folderPath = await invoke('select_video_folder');
      if (folderPath) {
        await runBatchImport([folderPath]);
      }
    } catch (e) {
      showToast('Error selecting folder: ' + e, 'error');
    } finally {
      if (btnImportFolder) {
        btnImportFolder.disabled = false;
        btnImportFolder.innerHTML = `<span id="icon-import-folder-slot">${Icons.folderPlus}</span> <span>Import Folder</span>`;
      }
    }
  }

  btnImport.addEventListener('click', triggerNativeFilePicker);

  if (btnImportFolder) {
    btnImportFolder.addEventListener('click', triggerNativeFolderPicker);
  }

  dropzone.addEventListener('click', (e) => {
    if (e.target.closest('#btn-browse-files') || e.target.closest('#btn-browse-folder')) {
      return;
    }
    triggerNativeFilePicker();
  });

  const btnBrowseFiles = document.getElementById('btn-browse-files');
  if (btnBrowseFiles) {
    btnBrowseFiles.addEventListener('click', (e) => {
      e.stopPropagation();
      triggerNativeFilePicker();
    });
  }

  const btnBrowseFolder = document.getElementById('btn-browse-folder');
  if (btnBrowseFolder) {
    btnBrowseFolder.addEventListener('click', (e) => {
      e.stopPropagation();
      triggerNativeFolderPicker();
    });
  }

  if (btnBatchClose) {
    btnBatchClose.addEventListener('click', () => {
      if (modalBatchImport) modalBatchImport.classList.add('hidden');
    });
  }

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
      await runBatchImport(paths);
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
  const getCloseBehavior = () => localStorage.getItem('close_behavior') || 'ask';

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

  // Reset prompt button to default (ask every time)
  if (btnResetClosePref) {
    btnResetClosePref.addEventListener('click', () => {
      localStorage.setItem('close_behavior', 'ask');
      updateSettingDropdown();
      showToast('Close behavior reset to Ask every time', 'info');
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

    // Ctrl+Q: Trigger safe close / quit flow
    if (e.ctrlKey && e.key.toLowerCase() === 'q') {
      e.preventDefault();
      triggerCloseFlow();
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
  const badgeAutostart = document.getElementById('badge-autostart');

  const updateAutostartUi = (enabled) => {
    chkAutostart.checked = enabled;
    if (labelAutostartStatus) {
      labelAutostartStatus.textContent = enabled ? 'Enabled' : 'Disabled';
      labelAutostartStatus.style.color = enabled ? '#10b981' : 'var(--text-secondary)';
    }
    if (badgeAutostart) {
      if (enabled) {
        badgeAutostart.classList.remove('settings-badge-inactive');
        badgeAutostart.classList.add('settings-badge-active');
      } else {
        badgeAutostart.classList.remove('settings-badge-active');
        badgeAutostart.classList.add('settings-badge-inactive');
      }
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

  // Start Minimized Control
  if (chkStartMinimized) {
    try {
      const isMinimized = await invoke('get_start_minimized');
      chkStartMinimized.checked = !!isMinimized;
    } catch (err) {
      console.error('Failed to query start minimized status:', err);
    }

    chkStartMinimized.addEventListener('change', async (e) => {
      const targetState = e.target.checked;
      try {
        const result = await invoke('set_start_minimized', { enable: targetState });
        chkStartMinimized.checked = result;
        showToast(result ? 'Start minimized enabled' : 'Start minimized disabled', 'info');
      } catch (err) {
        showToast(`Failed to update start minimized setting: ${err}`, 'error');
        try {
          const isMinimized = await invoke('get_start_minimized');
          chkStartMinimized.checked = !!isMinimized;
        } catch (_) {}
      }
    });
  }
}

// Application Menu Integration Controls
async function setupAppMenuControls() {
  const chkAppMenu = document.getElementById('chk-app-menu');
  const labelAppMenuStatus = document.getElementById('label-app-menu-status');
  const badgeAppMenu = document.getElementById('badge-app-menu');
  const btnAddAppMenu = document.getElementById('btn-add-app-menu');
  const btnRemoveAppMenu = document.getElementById('btn-remove-app-menu');
  const textAppMenuToggle = document.getElementById('text-app-menu-toggle');

  if (!chkAppMenu) return;

  const updateAppMenuUi = (inMenu) => {
    chkAppMenu.checked = inMenu;
    if (labelAppMenuStatus) {
      labelAppMenuStatus.textContent = inMenu ? 'In Application Menu' : 'Not in App Menu';
      labelAppMenuStatus.style.color = inMenu ? '#10b981' : 'var(--text-secondary)';
    }
    if (textAppMenuToggle) {
      textAppMenuToggle.textContent = inMenu ? 'In App Menu' : 'Show in Menu';
    }
    if (badgeAppMenu) {
      if (inMenu) {
        badgeAppMenu.classList.remove('settings-badge-inactive');
        badgeAppMenu.classList.add('settings-badge-active');
      } else {
        badgeAppMenu.classList.remove('settings-badge-active');
        badgeAppMenu.classList.add('settings-badge-inactive');
      }
    }
  };

  try {
    const inMenu = await invoke('get_app_menu_status');
    updateAppMenuUi(inMenu);
  } catch (err) {
    console.error('Failed to query app menu status:', err);
  }

  const setMenuState = async (targetState) => {
    try {
      const result = await invoke('set_app_menu_entry', { enable: targetState });
      updateAppMenuUi(result);
      if (result) {
        showToast('Animated Linux added to application menu', 'success');
      } else {
        showToast('Animated Linux removed from application menu', 'info');
      }
    } catch (err) {
      showToast(`Failed to update application menu entry: ${err}`, 'error');
      try {
        const inMenu = await invoke('get_app_menu_status');
        updateAppMenuUi(inMenu);
      } catch (_) {}
    }
  };

  chkAppMenu.addEventListener('change', (e) => setMenuState(e.target.checked));
  if (btnAddAppMenu) btnAddAppMenu.addEventListener('click', () => setMenuState(true));
  if (btnRemoveAppMenu) btnRemoveAppMenu.addEventListener('click', () => setMenuState(false));
}

// Wallpaper Engine Backend Controls
async function setupBackendControls() {
  if (!settingBackend) return;
  const labelActiveBackend = document.getElementById('label-active-backend');
  const descBackendMode = document.getElementById('desc-backend-mode');

  const backendInfo = {
    'kde_mpv': {
      title: 'KDE Plasma 6 (MPV Native Backend)',
      color: '#10b981',
      desc: 'KDE Plasma MPV embeds libmpv directly inside PlasmaShell underneath desktop icons with full hardware acceleration.'
    },
    'kde': {
      title: 'KDE Plasma 6 (QtMultimedia Reborn)',
      color: 'var(--text-primary)',
      desc: 'KDE Native QtMultimedia uses FFmpeg VA-API decoding inside PlasmaShell directly beneath desktop icons.'
    },
    'mpv': {
      title: 'Standalone MPV Engine (mpvpaper)',
      color: '#38bdf8',
      desc: 'Isolated Wayland layer surface with zero-copy DRM scanout (minimum CPU). Occludes desktop icons on Plasma.'
    }
  };

  const updateBackendLabel = (backend) => {
    const info = backendInfo[backend] || backendInfo['kde'];
    if (labelActiveBackend) {
      labelActiveBackend.textContent = info.title;
      labelActiveBackend.style.color = info.color;
    }
    if (descBackendMode) {
      descBackendMode.textContent = info.desc;
    }
  };

  try {
    const currentBackend = await invoke('get_wallpaper_backend');
    settingBackend.value = currentBackend;
    updateBackendLabel(currentBackend);
  } catch (err) {
    console.error('Failed to query current wallpaper backend:', err);
  }

  settingBackend.addEventListener('change', async (e) => {
    const selected = e.target.value;
    try {
      showToast('Switching wallpaper engine backend...', 'info');
      await invoke('set_wallpaper_backend', { backend: selected });
      updateBackendLabel(selected);
      const info = backendInfo[selected] || backendInfo['kde'];
      showToast(`Wallpaper engine switched to ${info.title}`, 'success');
    } catch (err) {
      showToast(`Failed to switch engine: ${err}`, 'error');
      try {
        const current = await invoke('get_wallpaper_backend');
        settingBackend.value = current;
        updateBackendLabel(current);
      } catch (_) {}
    }
  });
}

// Start Application
init();
