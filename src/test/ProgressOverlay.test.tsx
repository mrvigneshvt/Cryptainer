import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { ProgressOverlay } from '../components/UI';

describe('ProgressOverlay', () => {
  it('renders nothing when closed', () => {
    const { container } = render(
      <ProgressOverlay open={false} progress={null} />
    );
    expect(container.querySelector('.progress-overlay')).not.toBeInTheDocument();
  });

  it('renders ProgressBar inside a portal when open', () => {
    render(
      <ProgressOverlay
        open={true}
        progress={{
          operation: 'encrypt',
          current: 1,
          total: 3,
          file_name: 'a.txt',
          bytes_processed: 1024,
          bytes_total: 4096,
          message: 'Encrypting a.txt (1 / 3)',
        }}
      />
    );
    expect(screen.getByText('Encrypting')).toBeInTheDocument();
    expect(screen.getByText('a.txt')).toBeInTheDocument();
    expect(screen.getByText('1 / 3 files')).toBeInTheDocument();
  });

  it('shows fallbackMessage when progress is null', () => {
    render(
      <ProgressOverlay open={true} progress={null} fallbackMessage="Preparing\u2026" />
    );
    expect(screen.getByText((content) => content.includes('Preparing'))).toBeInTheDocument();
  });
});
