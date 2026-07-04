import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { ProgressBar } from '../components/UI'

describe('ProgressBar', () => {
  // ── Determinate mode ────────────────────────────────────────────────

  it('renders header verb + file name + count + percentage in determinate mode', () => {
    render(
      <ProgressBar
        operation="encrypt"
        current={3}
        total={12}
        fileName="vacation-2024.mp4"
      />
    )
    expect(screen.getByText('Encrypting')).toBeInTheDocument()
    expect(screen.getByText('vacation-2024.mp4')).toBeInTheDocument()
    expect(screen.getByText('3 / 12 files')).toBeInTheDocument()
    expect(screen.getByText('25%')).toBeInTheDocument()
  })

  it('renders bytes row when byte props provided', () => {
    render(
      <ProgressBar
        operation="encrypt"
        current={3}
        total={12}
        fileName="vacation-2024.mp4"
        bytesProcessed={10_485_760}   // 10 MiB → "10.0 MB"
        bytesTotal={31_457_280}       // 30 MiB → "30.0 MB"
        throughputBytesPerSec={5_242_880}  // 5 MiB/s → "5.0 MB/s"
        etaMs={2000}
      />
    )
    expect(screen.getByText(/10\.0 MB/)).toBeInTheDocument()
    expect(screen.getByText(/30\.0 MB/)).toBeInTheDocument()
    expect(screen.getByText(/5\.0 MB\/s/)).toBeInTheDocument()
    expect(screen.getByText(/~2s left/)).toBeInTheDocument()
  })

  it('hides bytes/speed row when byte props omitted', () => {
    const { container } = render(
      <ProgressBar
        operation="decrypt"
        current={1}
        total={5}
        fileName="doc.pdf"
      />
    )
    // No bytes or speed text
    expect(container.querySelector('.progress-bytes')).toBeNull()
  })

  it('renders the bar fill with correct width percentage', () => {
    const { container } = render(
      <ProgressBar
        operation="encrypt"
        current={3}
        total={12}
      />
    )
    const fill = container.querySelector('.progress-fill')
    expect(fill).toBeInTheDocument()
    expect(fill).toHaveStyle({ width: '25%' })
  })

  // ── Indeterminate mode ──────────────────────────────────────────────

  it('renders indeterminate shimmer when indeterminate prop is true', () => {
    const { container } = render(
      <ProgressBar
        operation="derive-key"
        current={0}
        total={0}
        indeterminate={true}
        message="Deriving encryption key…"
      />
    )
    expect(screen.getByText('Deriving encryption key…')).toBeInTheDocument()
    // No count, no percentage
    expect(screen.queryByText(/\d+\s*\/\s*\d+/)).toBeNull()
    expect(screen.queryByText(/\d+%/)).toBeNull()
    // Shimmer bar present
    expect(container.querySelector('.progress-shimmer')).toBeInTheDocument()
  })

  it('renders indeterminate shimmer when total is 0', () => {
    const { container } = render(
      <ProgressBar
        operation="derive-key"
        current={0}
        total={0}
        message="Working…"
      />
    )
    expect(container.querySelector('.progress-shimmer')).toBeInTheDocument()
  })

  // ── Error state ─────────────────────────────────────────────────────

  it('renders error state with red bar and error message', () => {
    const { container } = render(
      <ProgressBar
        operation="encrypt"
        current={0}
        total={12}
        error="Disk full"
      />
    )
    expect(screen.getByText('Disk full')).toBeInTheDocument()
    expect(container.querySelector('.progress-error')).toBeInTheDocument()
  })

  // ── Compact mode ────────────────────────────────────────────────────

  it('renders compact mode with fewer rows', () => {
    const { container } = render(
      <ProgressBar
        operation="encrypt"
        current={1}
        total={5}
        compact={true}
      />
    )
    expect(container.querySelector('.progress-compact')).toBeInTheDocument()
  })

  // ── Operation verb mapping ──────────────────────────────────────────

  it('maps operation to correct header text', () => {
    const { rerender } = render(
      <ProgressBar operation="encrypt" current={1} total={2} />
    )
    expect(screen.getByText('Encrypting')).toBeInTheDocument()

    rerender(<ProgressBar operation="decrypt" current={1} total={2} />)
    expect(screen.getByText('Decrypting')).toBeInTheDocument()

    rerender(<ProgressBar operation="derive-key" current={0} total={0} indeterminate />)
    expect(screen.getByText('Deriving key…')).toBeInTheDocument()

    rerender(<ProgressBar operation="write-blob" current={0} total={0} indeterminate />)
    expect(screen.getByText('Writing blob…')).toBeInTheDocument()

    rerender(<ProgressBar operation="read-blob" current={0} total={0} indeterminate />)
    expect(screen.getByText('Reading blob…')).toBeInTheDocument()

    rerender(<ProgressBar operation="migrate" current={0} total={0} indeterminate />)
    expect(screen.getByText('Migrating…')).toBeInTheDocument()

    rerender(<ProgressBar operation="import" current={0} total={0} indeterminate />)
    expect(screen.getByText('Importing')).toBeInTheDocument()

    rerender(<ProgressBar operation="export" current={0} total={0} indeterminate />)
    expect(screen.getByText('Exporting')).toBeInTheDocument()
  })

  // ── Zero total edge case ────────────────────────────────────────────

  it('handles 0% when current is 0 and total is 0 (indeterminate)', () => {
    const { container } = render(
      <ProgressBar
        operation="derive-key"
        current={0}
        total={0}
      />
    )
    // Should show shimmer, not a 0% fill
    expect(container.querySelector('.progress-shimmer')).toBeInTheDocument()
    expect(container.querySelector('.progress-fill')).toBeNull()
  })

  it('renders 100% when current equals total', () => {
    const { container } = render(
      <ProgressBar
        operation="encrypt"
        current={12}
        total={12}
        fileName="last.mp4"
      />
    )
    const fill = container.querySelector('.progress-fill')
    expect(fill).toHaveStyle({ width: '100%' })
    expect(screen.getByText('100%')).toBeInTheDocument()
  })
})
