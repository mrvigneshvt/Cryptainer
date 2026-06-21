import React from 'react';
import type { ContainerMeta } from '../../types/vault';
import { formatBytes } from '../../utils/format';
import './TerminalShell.css';

type SortOption = 'name' | 'date' | 'size' | 'files';

interface TerminalShellProps {
  /** Filtered + sorted containers to render as output blocks */
  containers: ContainerMeta[];
  /** Total registered containers (pre-filter) */
  totalCount: number;
  stats: { total: number; files: number; size: number; algorithms: string[] };
  searchQuery: string;
  setSearchQuery: (v: string) => void;
  sortBy: SortOption;
  setSortBy: (v: SortOption) => void;
  selectedTag: string | null;
  setSelectedTag: (v: string | null) => void;
  allTags: string[];
  loading: boolean;
  isImporting: boolean;
  onOpen: (c: ContainerMeta) => void;
  onExport: (c: ContainerMeta, e: React.MouseEvent) => void;
  onDelete: (c: ContainerMeta, e: React.MouseEvent) => void;
  onCreate: () => void;
  onImport: () => void;
}

/** project alpha → project_alpha — gives the hacker "filename" cadence */
const slug = (name: string) => name.toLowerCase().replace(/\s+/g, '_');

export const TerminalShell: React.FC<TerminalShellProps> = ({
  containers,
  totalCount,
  stats,
  searchQuery,
  setSearchQuery,
  sortBy,
  setSortBy,
  selectedTag,
  setSelectedTag,
  allTags,
  loading,
  isImporting,
  onOpen,
  onExport,
  onDelete,
  onCreate,
  onImport,
}) => {
  const cipher = stats.algorithms.length > 0 ? stats.algorithms[0] : 'none';

  return (
    <div className="tshell">
      {/* ── sticky header: window bar + command line + telemetry ─────────── */}
      <div className="tshell-head">
        <div className="tshell-titlebar">
          <span className="tshell-dots" aria-hidden="true">
            <i /><i /><i />
          </span>
          <span className="tshell-path">cryptainer://secure.shell</span>
          <span className="tshell-conn">
            <span className="tshell-conn-dot" /> connected
          </span>
        </div>

        <form className="tshell-cmd" onSubmit={(e) => e.preventDefault()} role="search">
          <span className="tshell-prompt">
            <span className="tshell-user">root@cryptainer</span>
            <span className="tshell-colon">:</span>
            <span className="tshell-cwd">~/vault</span>
            <span className="tshell-dollar">$</span>
            <span className="tshell-verb">grep</span>
          </span>
          <input
            className="tshell-input"
            type="text"
            value={searchQuery}
            spellCheck={false}
            autoComplete="off"
            placeholder="filter vaults by name or tag…"
            onChange={(e) => setSearchQuery(e.target.value)}
            aria-label="Filter vaults"
          />
          {searchQuery ? (
            <button type="button" className="tshell-clear" onClick={() => setSearchQuery('')}>
              ^C
            </button>
          ) : (
            <span className="tshell-caret" aria-hidden="true" />
          )}
        </form>

        <div className="tshell-telemetry" role="status">
          <span className="tshell-tel"><b>vaults</b> {stats.total}</span>
          <span className="tshell-tel"><b>files</b> {stats.files.toLocaleString()}</span>
          <span className="tshell-tel"><b>size</b> {formatBytes(stats.size)}</span>
          <span className="tshell-tel"><b>cipher</b> {cipher}</span>
          <span className="tshell-tel tshell-tel--ok"><span className="tshell-conn-dot" /> online</span>
          <span className="tshell-grow" />
          <button type="button" className="tshell-btn tshell-btn--ghost" onClick={onImport} disabled={isImporting}>
            {isImporting ? 'importing…' : 'import'}
          </button>
          <button type="button" className="tshell-btn" onClick={onCreate}>+ new vault</button>
        </div>

        {/* flags line: --sort / --tag */}
        <div className="tshell-flags">
          <span className="tshell-flag-key">--sort</span>
          <select
            className="tshell-select"
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as SortOption)}
            aria-label="Sort vaults"
          >
            <option value="date">date</option>
            <option value="name">name</option>
            <option value="size">size</option>
            <option value="files">files</option>
          </select>

          {allTags.length > 0 && (
            <>
              <span className="tshell-flag-key">--tag</span>
              {selectedTag && (
                <button className="tshell-tag tshell-tag--clear" onClick={() => setSelectedTag(null)}>
                  ×{selectedTag}
                </button>
              )}
              {allTags.map((tag) => (
                <button
                  key={tag}
                  className={`tshell-tag ${selectedTag === tag ? 'is-active' : ''}`}
                  onClick={() => setSelectedTag(selectedTag === tag ? null : tag)}
                >
                  {tag}
                </button>
              ))}
            </>
          )}
        </div>
      </div>

      {/* ── output stream ───────────────────────────────────────────────── */}
      <div className="tshell-output">
        {loading ? (
          <div className="tshell-status-line">
            establishing secure link<span className="tshell-caret" />
          </div>
        ) : totalCount === 0 ? (
          <div className="tshell-empty">
            <pre className="tshell-art">{String.raw`
   ┌───────────────┐
   │   [ LOCKED ]  │
   │    ▄▄▄▄▄▄▄    │
   │   █  ███  █   │
   │   █  ███  █   │
   └───────────────┘`}</pre>
            <div className="tshell-status-line">vault registry empty — no containers initialized.</div>
            <button className="tshell-btn" onClick={onCreate}>+ initialize vault</button>
          </div>
        ) : containers.length === 0 ? (
          <div className="tshell-empty">
            <div className="tshell-status-line">
              query <em>"{searchQuery || selectedTag}"</em> returned 0 results.
            </div>
            <button
              className="tshell-btn tshell-btn--ghost"
              onClick={() => {
                setSearchQuery('');
                setSelectedTag(null);
              }}
            >
              clear filters
            </button>
          </div>
        ) : (
          <>
            <div className="tshell-output-meta">
              {/* mimic a shell command echo */}
              <span className="tshell-output-echo">
                ls -l ~/vault {searchQuery && `| grep "${searchQuery}"`}{selectedTag && ` #${selectedTag}`}
              </span>
              <span className="tshell-output-count">{containers.length} match{containers.length !== 1 ? 'es' : ''}</span>
            </div>

            {containers.map((c, i) => {
              const tags = c.tags
                ? c.tags.split(',').map((t) => t.trim()).filter(Boolean)
                : [];
              return (
                <div
                  key={c.id}
                  className="tblock"
                  style={{ animationDelay: `${Math.min(i, 12) * 45}ms` }}
                  role="button"
                  tabIndex={0}
                  aria-label={`Open vault ${c.name}`}
                  onClick={() => onOpen(c)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      onOpen(c);
                    }
                  }}
                >
                  <div className="tblock-head">
                    <span className="tblock-marker">[OK]</span>
                    <span className="tblock-name">
                      vault.<b>{slug(c.name)}</b>
                    </span>
                    <span className="tblock-grow" />
                    <span className="tblock-algo">{c.algo}</span>
                  </div>

                  <div className="tblock-body">
                    <div className="tblock-row">
                      <span className="tblock-tree">├─</span>
                      <span className="tblock-k">size</span>
                      <span className="tblock-v">{c.file_count} files · {formatBytes(c.total_size)}</span>
                    </div>
                    <div className="tblock-row">
                      <span className="tblock-tree">├─</span>
                      <span className="tblock-k">born</span>
                      <span className="tblock-v">{new Date(c.created_at).toLocaleDateString()}</span>
                      {tags.length > 0 && (
                        <span className="tblock-tags">
                          {tags.map((t) => (
                            <span key={t} className="tblock-tag">#{t}</span>
                          ))}
                        </span>
                      )}
                    </div>
                    <div className="tblock-row tblock-row--last">
                      <span className="tblock-tree">└─</span>
                      <span className="tblock-k">sha256</span>
                      <span className="tblock-hash" title={c.blob_sha256}>
                        {c.blob_sha256.substring(0, 24)}…
                      </span>
                      <span className="tblock-grow" />
                      <span className="tblock-actions">
                        <button
                          className="tblock-cmd"
                          onClick={(e) => {
                            e.stopPropagation();
                            onOpen(c);
                          }}
                        >
                          open
                        </button>
                        <button className="tblock-cmd" onClick={(e) => onExport(c, e)}>
                          export
                        </button>
                        <button className="tblock-cmd tblock-cmd--del" onClick={(e) => onDelete(c, e)}>
                          rm
                        </button>
                      </span>
                    </div>
                  </div>
                </div>
              );
            })}

            <div className="tshell-eof">
              <span className="tshell-caret" /> end of stream — {containers.length} vault{containers.length !== 1 ? 's' : ''} listed
            </div>
          </>
        )}
      </div>
    </div>
  );
};
