import { useEffect, useMemo, useState } from 'react';
import { useVaultStore } from './store/vaultStore';
import { CreateWizard } from './components/Container/CreateWizard';
import { ContainerModal } from './components/Container/ContainerModal';
import { Settings } from './components/Settings';
import { useAutoLock } from './hooks/useAutoLock';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { ContainerMeta } from './types/vault';
import { formatBytes } from './utils/format';
import './App.css';

type SortOption = 'name' | 'date' | 'size' | 'files';

function App() {
  const { containers, loading, error, loadContainers, clearError, importContainer, exportContainer, deleteContainer } = useVaultStore();
  const [showCreate, setShowCreate] = useState(false);
  const [activeContainer, setActiveContainer] = useState<ContainerMeta | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  
  // Search and filter states
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<SortOption>('date');

  // Auto-lock hook
  const { isLocked: _isLocked } = useAutoLock(() => {
    // Lock all containers on auto-lock
    if (activeContainer) {
      setActiveContainer(null);
    }
  });

  useEffect(() => {
    loadContainers();
  }, []);

  // Extract all unique tags from containers
  const allTags = useMemo(() => {
    const tagSet = new Set<string>();
    containers.forEach(c => {
      if (c.tags) {
        c.tags.split(',').forEach(t => tagSet.add(t.trim()));
      }
    });
    return Array.from(tagSet).sort();
  }, [containers]);

  // Filter and sort containers
  const filteredContainers = useMemo(() => {
    let result = [...containers];
    
    // Filter by search query
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      result = result.filter(c => 
        c.name.toLowerCase().includes(query) ||
        (c.tags && c.tags.toLowerCase().includes(query))
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
        case 'name':
          return a.name.localeCompare(b.name);
        case 'date':
          return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
        case 'size':
          return b.total_size - a.total_size;
        case 'files':
          return b.file_count - a.file_count;
        default:
          return 0;
      }
    });
    
    return result;
  }, [containers, searchQuery, selectedTag, sortBy]);

  const handleImport = async () => {
    setIsImporting(true);
    try {
      const selected = await open({
        multiple: true,
        filters: [{
          name: 'Cryptainer Export',
          extensions: ['ctnr']
        }]
      });
      
      if (selected && Array.isArray(selected)) {
        let successCount = 0;
        let failCount = 0;
        for (const path of selected) {
          try {
            await importContainer(path);
            successCount++;
          } catch {
            failCount++;
          }
        }
        if (failCount > 0) {
          const msg = failCount === selected.length
            ? `Import failed for all ${failCount} files`
            : `Imported ${successCount} file(s), ${failCount} failed`;
          console.warn(msg);
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
        filters: [{
          name: 'Cryptainer Export',
          extensions: ['ctnr']
        }],
        defaultPath: `${container.name}.ctnr`
      });
      if (path) {
        await exportContainer(container.id, path);
      }
    } catch {
      // Error handled by vaultStore
    }
  };

  const handleDelete = async (container: ContainerMeta, e: React.MouseEvent) => {
    e.stopPropagation();
    if (confirm(`Delete "${container.name}"? This cannot be undone.`)) {
      try {
        await deleteContainer(container.id);
      } catch {
        // Error handled by vaultStore
      }
    }
  };

  return (
    <div className="app">
      <header className="app-header">
        <div className="logo">CRYPTAINER</div>
        <div className="header-actions">
          <button 
            className="btn-ghost" 
            onClick={() => setShowSettings(true)}
            title="Settings"
          >
            ⚙️
          </button>
          <button 
            className="btn-secondary" 
            onClick={handleImport}
            disabled={isImporting}
          >
            {isImporting ? 'Importing...' : 'Import .ctnr'}
          </button>
          <button className="btn-primary" onClick={() => setShowCreate(true)}>
            + New Container
          </button>
        </div>
      </header>
      
      <div className="vault-toolbar">
        <div className="search-box">
          <input
            type="text"
            placeholder="Search containers..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="search-input"
          />
          {searchQuery && (
            <button className="search-clear" onClick={() => setSearchQuery('')}>
              ×
            </button>
          )}
        </div>
        
        <select 
          className="sort-select"
          value={sortBy}
          onChange={(e) => setSortBy(e.target.value as SortOption)}
        >
          <option value="date">Sort by Date</option>
          <option value="name">Sort by Name</option>
          <option value="size">Sort by Size</option>
          <option value="files">Sort by Files</option>
        </select>
      </div>
      
      {allTags.length > 0 && (
        <div className="tag-filter">
          <span className="tag-label">Filter by tag:</span>
          <div className="tag-list">
            {selectedTag && (
              <button 
                className="tag-btn clear"
                onClick={() => setSelectedTag(null)}
              >
                Clear
              </button>
            )}
            {allTags.map(tag => (
              <button
                key={tag}
                className={`tag-btn ${selectedTag === tag ? 'active' : ''}`}
                onClick={() => setSelectedTag(selectedTag === tag ? null : tag)}
              >
                {tag}
              </button>
            ))}
          </div>
        </div>
      )}
      
      <main className="app-main">
        {loading ? (
          <div className="loading">Loading vault…</div>
        ) : containers.length === 0 ? (
          <div className="empty-state">
            <div className="empty-icon">🔐</div>
            <h2>Your vault is empty</h2>
            <p>Create your first encrypted container to get started.</p>
            <button className="btn-primary" onClick={() => setShowCreate(true)}>
              Create Container
            </button>
          </div>
        ) : filteredContainers.length === 0 ? (
          <div className="empty-state">
            <div className="empty-icon">🔍</div>
            <h2>No containers found</h2>
            <p>Try adjusting your search or filters.</p>
            {(searchQuery || selectedTag) && (
              <button 
                className="btn-secondary" 
                onClick={() => { setSearchQuery(''); setSelectedTag(null); }}
              >
                Clear filters
              </button>
            )}
          </div>
        ) : (
          <div className="vault-grid">
            {filteredContainers.map(container => (
              <div 
                key={container.id} 
                className="container-card"
                onClick={() => setActiveContainer(container)}
              >
                <div className="card-header">
                  <span className="algo-badge">{container.algo}</span>
                  <div className="card-actions">
                    <button 
                      className="card-action-btn"
                      onClick={(e) => handleExport(container, e)}
                      title="Export"
                    >
                      ↓
                    </button>
                    <button 
                      className="card-action-btn delete"
                      onClick={(e) => handleDelete(container, e)}
                      title="Delete"
                    >
                      ×
                    </button>
                  </div>
                </div>
                <h3 className="card-title">{container.name}</h3>
                <div className="card-meta">
                  <span>{container.file_count} files · {formatBytes(container.total_size)}</span>
                  <span>{new Date(container.created_at).toLocaleDateString()}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </main>

      {error && (
        <div className="error-toast" onClick={clearError}>
          {error}
        </div>
      )}

      {/* Modals */}
      {showCreate && (
        <CreateWizard onClose={() => setShowCreate(false)} />
      )}
      
      {activeContainer && (
        <ContainerModal
          container={activeContainer}
          onClose={() => setActiveContainer(null)}
        />
      )}
      
      {showSettings && (
        <Settings onClose={() => setShowSettings(false)} />
      )}
    </div>
  );
}

export default App;
