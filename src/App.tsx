import { useEffect, useMemo, useState } from 'react';
import { useVaultStore } from './store/vaultStore';
import { CreateWizard } from './components/Container/CreateWizard';
import { ContainerModal } from './components/Container/ContainerModal';
import { Settings } from './components/Settings';
import { ThemeToggle } from './components/UI/ThemeToggle';
import { useAutoLock } from './hooks/useAutoLock';
import { useMediaQuery } from './hooks/useMediaQuery';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { ContainerMeta } from './types/vault';
import { formatBytes } from './utils/format';
import './App.css';

type SortOption = 'name' | 'date' | 'size' | 'files';
type NavSection = 'containers' | 'settings';

function App() {
  const { containers, loading, error, loadContainers, clearError, importContainer, exportContainer, deleteContainer } = useVaultStore();
  const [showCreate, setShowCreate] = useState(false);
  const [activeContainer, setActiveContainer] = useState<ContainerMeta | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [activeNav, setActiveNav] = useState<NavSection>('containers');
  const [showSettings, setShowSettings] = useState(false);

  // Responsive sidebar state
  const { isMobile, isTablet } = useMediaQuery();
  const isSmallScreen = isMobile || isTablet;

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

    // Filter by search query
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(c =>
        c.name.toLowerCase().includes(q) ||
        (c.tags && c.tags.toLowerCase().includes(q))
      );
    }

    // Filter by selected tag
    if (selectedTag) {
      result = result.filter(c =>
        c.tags && c.tags.split(',').some(t => t.trim() === selectedTag)
      );
    }

    // Sort
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
    algorithms: [...new Set(containers.map(c => c.algo))].join(', '),
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

  const handleDelete = async (container: ContainerMeta, e: React.MouseEvent) => {
    e.stopPropagation();
    if (confirm(`Delete "${container.name}"? This cannot be undone.`)) {
      try { await deleteContainer(container.id); }
      catch { /* handled by vaultStore */ }
    }
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
      {/* ============ SIDEBAR (desktop only) ============ */}
      {!isSmallScreen && (
        <aside
          className="sidebar"
          role="navigation"
          aria-label="Main navigation"
        >
          <div className="sidebar-logo">
            <svg className="sidebar-logo-icon" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
              <path d="M7 11V7a5 5 0 0 1 10 0v4" />
            </svg>
            <span className="sidebar-logo-text">Cryptainer</span>
          </div>

          <nav className="sidebar-nav">
            <div className="sidebar-section-label">Navigation</div>
            <button
              className={`sidebar-nav-item ${activeNav === 'containers' ? 'active' : ''}`}
              onClick={() => setActiveNav('containers')}
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M22 12h-4l-3 9-4-18-3 9H2" />
              </svg>
              <span>Containers</span>
              <span className="sidebar-badge">{containers.length}</span>
            </button>

            <button
              className="sidebar-nav-item"
              onClick={handleImport}
              disabled={isImporting}
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
              </svg>
              <span>Import</span>
            </button>

            <button
              className="sidebar-nav-item"
              onClick={() => setShowCreate(true)}
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <line x1="12" y1="5" x2="12" y2="19" />
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
              <span>New Container</span>
            </button>
          </nav>

          <div className="sidebar-divider" />

          <div className="sidebar-nav">
            <div className="sidebar-section-label">System</div>
            <button
              className="sidebar-nav-item"
              onClick={() => setShowSettings(true)}
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="3" />
                <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
              </svg>
              <span>Settings</span>
            </button>
          </div>

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
        {/* Topbar */}
        <div className="topbar">
          <div className="topbar-left">
            <h1 className="topbar-title">Containers</h1>
          </div>
          <div className="topbar-right">
            <div className="topbar-search">
              <svg className="topbar-search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              <input
                type="text"
                placeholder="Search containers..."
                value={searchQuery}
                onChange={e => setSearchQuery(e.target.value)}
                className="topbar-search-input"
              />
              {searchQuery && (
                <button className="topbar-search-clear" onClick={() => setSearchQuery('')}>×</button>
              )}
            </div>
            {!isSmallScreen && (
              <button className="btn-primary topbar-btn" onClick={() => setShowCreate(true)}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
                </svg>
                New
              </button>
            )}
          </div>
        </div>

        {/* Stats Row */}
        <div className="stats-row">
          <div className="stat-card">
            <span className="stat-value">{stats.total}</span>
            <span className="stat-label">Containers</span>
          </div>
          <div className="stat-card">
            <span className="stat-value">{stats.files.toLocaleString()}</span>
            <span className="stat-label">Files</span>
          </div>
          <div className="stat-card">
            <span className="stat-value">{formatBytes(stats.size)}</span>
            <span className="stat-label">Total Size</span>
          </div>
          <div className="stat-card">
            <span className="stat-value">{stats.algorithms || '—'}</span>
            <span className="stat-label">Algorithm</span>
          </div>
        </div>

        {/* Toolbar */}
        <div className="toolbar-row">
          {allTags.length > 0 && (
            <div className="toolbar-tags">
              <span className="toolbar-tag-label">Tags:</span>
              {selectedTag && (
                <button className="tag-chip tag-chip-clear" onClick={() => setSelectedTag(null)}>
                  Clear ×
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
            <div className="loading">Loading vault…</div>
          ) : containers.length === 0 ? (
            <div className="empty-state">
              <div className="empty-icon">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                  <path d="M7 11V7a5 5 0 0 1 10 0v4" />
                </svg>
              </div>
              <h2>Your vault is empty</h2>
              <p>Create your first encrypted container to get started.</p>
              <button className="btn-primary" onClick={() => setShowCreate(true)}>
                Create Container
              </button>
            </div>
          ) : filteredContainers.length === 0 ? (
            <div className="empty-state">
              <div className="empty-icon">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="11" cy="11" r="8" />
                  <line x1="21" y1="21" x2="16.65" y2="16.65" />
                </svg>
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
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
                          </svg>
                        </button>
                        <button className="bento-action-btn bento-action-delete" onClick={e => handleDelete(container, e)} title="Delete" aria-label="Delete">
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                            <polyline points="3 6 5 6 21 6" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                          </svg>
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
      </div>

      {/* ============ BOTTOM TAB BAR (mobile only) ============ */}
      {isSmallScreen && (
        <nav className="bottom-tabs" role="navigation" aria-label="Main navigation">
          <button
            className={`bottom-tab ${activeNav === 'containers' ? 'active' : ''}`}
            onClick={() => setActiveNav('containers')}
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M22 12h-4l-3 9-4-18-3 9H2" />
            </svg>
            <span>Containers</span>
          </button>

          <button
            className="bottom-tab"
            onClick={handleImport}
            disabled={isImporting}
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
            <span>Import</span>
          </button>

          <button
            className="bottom-tab bottom-tab-primary"
            onClick={() => setShowCreate(true)}
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            <span>New</span>
          </button>

          <button
            className="bottom-tab"
            onClick={() => setShowSettings(true)}
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="3" />
              <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
            </svg>
            <span>Settings</span>
          </button>

          <div className="bottom-tab-theme">
            <ThemeToggle />
          </div>
        </nav>
      )}

      {error && (
        <div className="error-toast" onClick={clearError}>
          {error}
        </div>
      )}

      {/* Modals */}
      {showCreate && <CreateWizard onClose={() => setShowCreate(false)} />}
      {activeContainer && (
        <ContainerModal
          container={activeContainer}
          onClose={() => setActiveContainer(null)}
        />
      )}
      {showSettings && <Settings onClose={() => setShowSettings(false)} />}
    </div>
  );
}

export default App;
