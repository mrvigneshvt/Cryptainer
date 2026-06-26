import { useEffect, useMemo, useState } from 'react';
import { useVaultStore } from './store/vaultStore';
import { CreateWizard } from './components/Container/CreateWizard';
import { ContainerModal } from './components/Container/ContainerModal';
import { Settings } from './components/Settings';
import { ConfirmModal } from './components/UI';
import { ThemeToggle } from './components/UI/ThemeToggle';
import { TerminalShell } from './components/Terminal/TerminalShell';
import { useThemeStore } from './store/themeStore';
import { useAutoLock } from './hooks/useAutoLock';
import { useMediaQuery } from './hooks/useMediaQuery';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { ContainerMeta } from './types/vault';
import { formatBytes } from './utils/format';
import './App.css';

type SortOption = 'name' | 'date' | 'size' | 'files';
type NavSection = 'containers' | 'settings';

/* ── Icon helpers ─────────────────────────────────────────────────────────── */
const IconLock = ({ size = 18 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect x="3" y="11" width="18" height="11" rx="2" ry="2" /><path d="M7 11V7a5 5 0 0 1 10 0v4" />
  </svg>
);
const IconActivity = ({ size = 18 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M22 12h-4l-3 9-4-18-3 9H2" />
  </svg>
);
const IconDownload = ({ size = 18 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
  </svg>
);
const IconPlus = ({ size = 18 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
  </svg>
);
const IconSettings = ({ size = 18 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="3" />
    <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
  </svg>
);
const IconSearch = ({ size = 16 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
  </svg>
);
const IconContainers = ({ size = 20 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect x="2" y="3" width="20" height="7" rx="2" ry="2" /><rect x="2" y="14" width="20" height="7" rx="2" ry="2" />
  </svg>
);
const IconFiles = ({ size = 20 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" /><polyline points="14 2 14 8 20 8" /><line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" /><polyline points="10 9 9 9 8 9" />
  </svg>
);
const IconHardDrive = ({ size = 20 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <line x1="22" y1="12" x2="2" y2="12" /><path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" /><line x1="6" y1="16" x2="6.01" y2="16" /><line x1="10" y1="16" x2="10.01" y2="16" />
  </svg>
);
const IconShield = ({ size = 20 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
  </svg>
);
const IconX = ({ size = 14 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
  </svg>
);
const IconExport = ({ size = 14 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
  </svg>
);
const IconTrash = ({ size = 14 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="3 6 5 6 21 6" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
  </svg>
);

function App() {
  const { containers, loading, error, isMock, loadContainers, clearError, importContainer, exportContainer, deleteContainer } = useVaultStore();
  const [showCreate, setShowCreate] = useState(false);
  const [activeContainer, setActiveContainer] = useState<ContainerMeta | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [activeNav, setActiveNav] = useState<NavSection>('containers');
  const [showSettings, setShowSettings] = useState(false);

  // Responsive sidebar state
  const { isMobile, isTablet } = useMediaQuery();
  const isSmallScreen = isMobile || isTablet;

  // Active theme — terminal theme swaps the dashboard for a hybrid CLI shell
  const theme = useThemeStore((s) => s.theme);
  const isTerminal = theme === 'terminal';

  // Search and filter states
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<SortOption>('date');

  // Auto-lock hook
  const { isLocked: _isLocked } = useAutoLock(() => {
    if (activeContainer) setActiveContainer(null);
  });

  useEffect(() => {
    loadContainers();
  }, []);

  // Extract all unique tags from containers
  const allTags = useMemo(() => {
    const tagSet = new Set<string>();
    containers.forEach(c => {
      if (c.tags) c.tags.split(',').forEach(t => tagSet.add(t.trim()));
    });
    return Array.from(tagSet).sort();
  }, [containers]);

  // Filter and sort containers
  const filteredContainers = useMemo(() => {
    let result = [...containers];

    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(c =>
        c.name.toLowerCase().includes(q) ||
        (c.tags && c.tags.toLowerCase().includes(q))
      );
    }

    if (selectedTag) {
      result = result.filter(c =>
        c.tags && c.tags.split(',').some(t => t.trim() === selectedTag)
      );
    }

    result.sort((a, b) => {
      switch (sortBy) {
        case 'name': return a.name.localeCompare(b.name);
        case 'date': return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
        case 'size': return b.total_size - a.total_size;
        case 'files': return b.file_count - a.file_count;
        default: return 0;
      }
    });

    return result;
  }, [containers, searchQuery, selectedTag, sortBy]);

  // Stats derived from real data
  const stats = useMemo(() => ({
    total: containers.length,
    files: containers.reduce((sum, c) => sum + c.file_count, 0),
    size: containers.reduce((sum, c) => sum + c.total_size, 0),
    algorithms: [...new Set(containers.map(c => c.algo))],
  }), [containers]);

  const handleImport = async () => {
    setIsImporting(true);
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: 'Cryptainer Export', extensions: ['ctnr'] }]
      });
      if (selected && Array.isArray(selected)) {
        let successCount = 0, failCount = 0;
        for (const path of selected) {
          try { await importContainer(path); successCount++; }
          catch { failCount++; }
        }
        if (failCount > 0) {
          console.warn(failCount === selected.length
            ? `Import failed for all ${failCount} files`
            : `Imported ${successCount} file(s), ${failCount} failed`);
        }
      }
    } catch (e) {
      console.error('Import cancelled:', e);
    } finally {
      setIsImporting(false);
    }
  };

  const handleExport = async (container: ContainerMeta, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const path = await save({
        filters: [{ name: 'Cryptainer Export', extensions: ['ctnr'] }],
        defaultPath: `${container.name}.ctnr`
      });
      if (path) await exportContainer(container.id, path);
    } catch { /* handled by vaultStore */ }
  };

  const [pendingDelete, setPendingDelete] = useState<ContainerMeta | null>(null);

  const handleDelete = (container: ContainerMeta, e: React.MouseEvent) => {
    e.stopPropagation();
    setPendingDelete(container);
  };

  const handleConfirmDelete = async () => {
    if (!pendingDelete) return;
    try { await deleteContainer(pendingDelete.id); }
    catch { /* handled by vaultStore */ }
    setPendingDelete(null);
  };

  const handleCancelDelete = () => {
    setPendingDelete(null);
  };

  const handleCardKeyDown = (container: ContainerMeta, e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setActiveContainer(container);
    }
  };

  // Determine featured (largest) container
  const featuredContainer = useMemo(() => {
    if (filteredContainers.length === 0) return null;
    return filteredContainers.reduce((a, b) => a.total_size > b.total_size ? a : b);
  }, [filteredContainers]);

  return (
    <div className="app">
      {/* Dev-mode banner */}
      {isMock && (
        <div className="dev-banner">
          <span className="dev-banner-dot" />
          Preview mode — running without Tauri backend
        </div>
      )}

      {/* ============ SIDEBAR (desktop only) ============ */}
      {!isSmallScreen && (
        <aside className="sidebar" role="navigation" aria-label="Main navigation">
          <div className="sidebar-logo">
            <div className="sidebar-logo-icon-wrap">
              <IconLock size={22} />
            </div>
            <span className="sidebar-logo-text">Cryptainer</span>
          </div>

          <nav className="sidebar-nav">
            <div className="sidebar-section-label">Navigation</div>
            <button
              className={`sidebar-nav-item ${activeNav === 'containers' ? 'active' : ''}`}
              onClick={() => setActiveNav('containers')}
            >
              <IconActivity size={18} />
              <span>Containers</span>
              <span className="sidebar-badge">{containers.length}</span>
            </button>

            <button className="sidebar-nav-item" onClick={handleImport} disabled={isImporting}>
              <IconDownload size={18} />
              <span>Import</span>
            </button>

            <button className="sidebar-nav-item" onClick={() => setShowCreate(true)}>
              <IconPlus size={18} />
              <span>New Container</span>
            </button>
          </nav>

          <div className="sidebar-divider" />

          <nav className="sidebar-nav">
            <div className="sidebar-section-label">System</div>
            <button className="sidebar-nav-item" onClick={() => setShowSettings(true)}>
              <IconSettings size={18} />
              <span>Settings</span>
            </button>
          </nav>

          <div className="sidebar-footer">
            <ThemeToggle />
            <div className="sidebar-status">
              <span className="sidebar-status-dot" />
              <span>{containers.length} container{containers.length !== 1 ? 's' : ''}</span>
            </div>
          </div>
        </aside>
      )}

      {/* ============ MAIN CONTENT ============ */}
      <div className={`main-content ${isSmallScreen ? 'main-content-mobile' : ''}`}>
        {isTerminal ? (
          <TerminalShell
            containers={filteredContainers}
            totalCount={containers.length}
            stats={stats}
            searchQuery={searchQuery}
            setSearchQuery={setSearchQuery}
            sortBy={sortBy}
            setSortBy={setSortBy}
            selectedTag={selectedTag}
            setSelectedTag={setSelectedTag}
            allTags={allTags}
            loading={loading}
            isImporting={isImporting}
            onOpen={setActiveContainer}
            onExport={handleExport}
            onDelete={handleDelete}
            onCreate={() => setShowCreate(true)}
            onImport={handleImport}
          />
        ) : (
        <>
        {/* Topbar */}
        <div className="topbar">
          <div className="topbar-left">
            <h1 className="topbar-title">Containers</h1>
            <span className="topbar-subtitle">{containers.length} total</span>
          </div>
          <div className="topbar-right">
            <div className="topbar-search">
              <IconSearch size={16} />
              <input
                type="text"
                placeholder="Search containers..."
                value={searchQuery}
                onChange={e => setSearchQuery(e.target.value)}
                className="topbar-search-input"
              />
              {searchQuery && (
                <button className="topbar-search-clear" onClick={() => setSearchQuery('')}>
                  <IconX size={14} />
                </button>
              )}
            </div>
            {!isSmallScreen && (
              <button className="btn-primary topbar-btn" onClick={() => setShowCreate(true)}>
                <IconPlus size={16} />
                New
              </button>
            )}
          </div>
        </div>

        {/* Stats Row */}
        <div className="stats-row">
          <div className="stat-card">
            <div className="stat-card-icon"><IconContainers size={20} /></div>
            <div className="stat-card-body">
              <span className="stat-value">{stats.total}</span>
              <span className="stat-label">Containers</span>
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-card-icon"><IconFiles size={20} /></div>
            <div className="stat-card-body">
              <span className="stat-value">{stats.files.toLocaleString()}</span>
              <span className="stat-label">Files</span>
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-card-icon"><IconHardDrive size={20} /></div>
            <div className="stat-card-body">
              <span className="stat-value">{formatBytes(stats.size)}</span>
              <span className="stat-label">Total Size</span>
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-card-icon"><IconShield size={20} /></div>
            <div className="stat-card-body">
              <span className="stat-value">
                {stats.algorithms.length > 0 ? stats.algorithms[0] : 'None'}
              </span>
              <span className="stat-label">
                {stats.algorithms.length > 1 ? `+${stats.algorithms.length - 1} more` : 'Algorithm'}
              </span>
            </div>
          </div>
        </div>

        {/* Toolbar */}
        <div className="toolbar-row">
          {allTags.length > 0 && (
            <div className="toolbar-tags">
              <span className="toolbar-tag-label">Tags:</span>
              {selectedTag && (
                <button className="tag-chip tag-chip-clear" onClick={() => setSelectedTag(null)}>
                  Clear <IconX size={12} />
                </button>
              )}
              {allTags.map(tag => (
                <button
                  key={tag}
                  className={`tag-chip ${selectedTag === tag ? 'active' : ''}`}
                  onClick={() => setSelectedTag(selectedTag === tag ? null : tag)}
                >
                  {tag}
                </button>
              ))}
            </div>
          )}
          <div className="toolbar-sort">
            <label htmlFor="sort-select" className="toolbar-sort-label">Sort:</label>
            <select
              id="sort-select"
              className="toolbar-sort-select"
              value={sortBy}
              onChange={e => setSortBy(e.target.value as SortOption)}
            >
              <option value="date">Date</option>
              <option value="name">Name</option>
              <option value="size">Size</option>
              <option value="files">Files</option>
            </select>
          </div>
        </div>

        {/* Main Content */}
        <main className="bento-main">
          {loading ? (
            <div className="loading">
              <div className="loading-spinner" />
              <span>Loading vault…</span>
            </div>
          ) : containers.length === 0 ? (
            <div className="empty-state">
              <div className="empty-illustration">
                <div className="empty-illustration-ring">
                  <IconLock size={40} />
                </div>
              </div>
              <h2>Your vault is empty</h2>
              <p>Create your first encrypted container to start securing files.</p>
              <button className="btn-primary" onClick={() => setShowCreate(true)}>
                <IconPlus size={16} />
                Create Container
              </button>
            </div>
          ) : filteredContainers.length === 0 ? (
            <div className="empty-state">
              <div className="empty-illustration">
                <div className="empty-illustration-ring empty-illustration-ring--muted">
                  <IconSearch size={40} />
                </div>
              </div>
              <h2>No containers found</h2>
              <p>Try adjusting your search or filters.</p>
              {(searchQuery || selectedTag) && (
                <button className="btn-secondary" onClick={() => { setSearchQuery(''); setSelectedTag(null); }}>
                  Clear filters
                </button>
              )}
            </div>
          ) : (
            <div className="bento-grid">
              {filteredContainers.map((container, index) => {
                const isFeatured = featuredContainer && container.id === featuredContainer.id;
                const staggerIndex = Math.min(index + 1, 8);
                return (
                  <div
                    key={container.id}
                    className={`bento-card ${isFeatured ? 'bento-featured' : ''} animate-scaleIn stagger-${staggerIndex}`}
                    onClick={() => setActiveContainer(container)}
                    role="button"
                    tabIndex={0}
                    onKeyDown={e => handleCardKeyDown(container, e)}
                    aria-label={`Container ${container.name}`}
                  >
                    <div className="bento-card-top">
                      <span className="algo-badge">{container.algo}</span>
                      <div className="bento-card-actions">
                        <button className="bento-action-btn" onClick={e => handleExport(container, e)} title="Export" aria-label="Export">
                          <IconExport size={14} />
                        </button>
                        <button className="bento-action-btn bento-action-delete" onClick={e => handleDelete(container, e)} title="Delete" aria-label="Delete">
                          <IconTrash size={14} />
                        </button>
                      </div>
                    </div>
                    <h3 className="bento-card-title">{container.name}</h3>
                    <div className="bento-card-stats">
                      <div className="bento-card-stat">
                        <span className="bento-card-stat-value">{container.file_count}</span>
                        <span className="bento-card-stat-label">files</span>
                      </div>
                      <div className="bento-card-stat">
                        <span className="bento-card-stat-value">{formatBytes(container.total_size)}</span>
                        <span className="bento-card-stat-label">size</span>
                      </div>
                      <div className="bento-card-stat">
                        <span className="bento-card-stat-value">{new Date(container.created_at).toLocaleDateString()}</span>
                        <span className="bento-card-stat-label">created</span>
                      </div>
                    </div>
                    {container.tags && (
                      <div className="bento-card-tags">
                        {container.tags.split(',').map(t => t.trim()).filter(Boolean).map(tag => (
                          <span key={tag} className="bento-card-tag">{tag}</span>
                        ))}
                      </div>
                    )}
                    <div className="bento-card-hash" title={container.blob_sha256}>
                      {container.blob_sha256.substring(0, 16)}…
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </main>
        </>
        )}
      </div>

      {/* ============ BOTTOM TAB BAR (mobile only) ============ */}
      {isSmallScreen && (
        <nav className="bottom-tabs" role="navigation" aria-label="Main navigation">
          <button className={`bottom-tab ${activeNav === 'containers' ? 'active' : ''}`} onClick={() => setActiveNav('containers')}>
            <IconActivity size={20} />
            <span>Containers</span>
          </button>
          <button className="bottom-tab" onClick={handleImport} disabled={isImporting}>
            <IconDownload size={20} />
            <span>Import</span>
          </button>
          <button className="bottom-tab bottom-tab-primary" onClick={() => setShowCreate(true)}>
            <IconPlus size={20} />
            <span>New</span>
          </button>
          <button className="bottom-tab" onClick={() => setShowSettings(true)}>
            <IconSettings size={20} />
            <span>Settings</span>
          </button>
          <div className="bottom-tab-theme">
            <ThemeToggle />
          </div>
        </nav>
      )}

      {/* Error Toast */}
      {error && (
        <div className="error-toast">
          <span className="error-toast-text">{error}</span>
          <button className="error-toast-close" onClick={clearError} aria-label="Dismiss error">
            <IconX size={14} />
          </button>
        </div>
      )}

      {/* Modals */}
      {showCreate && <CreateWizard onClose={() => setShowCreate(false)} />}
      {activeContainer && (
        <ContainerModal container={activeContainer} onClose={() => setActiveContainer(null)} />
      )}
      {showSettings && <Settings onClose={() => setShowSettings(false)} />}
      <ConfirmModal
        open={!!pendingDelete}
        danger
        title="Delete container"
        message={`Delete "${pendingDelete?.name ?? ''}"? This cannot be undone.`}
        confirmLabel="Delete"
        onConfirm={handleConfirmDelete}
        onCancel={handleCancelDelete}
      />
    </div>
  );
}

export default App;
