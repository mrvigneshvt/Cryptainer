import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Step1Files } from '../components/Container/CreateWizard/Step1Files';

// Drive the native picker instead of a drag-drop: stub the Tauri plugins.
const open = vi.fn();
const stat = vi.fn();
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: (...a: unknown[]) => open(...a) }));
vi.mock('@tauri-apps/plugin-fs', () => ({ stat: (...a: unknown[]) => stat(...a) }));

describe('Step1Files', () => {
  beforeEach(() => {
    open.mockReset();
    stat.mockReset();
    stat.mockResolvedValue({ size: 2048 });
  });

  it('adds a picked file to the list and passes path-based FileInputs to onNext', async () => {
    open.mockResolvedValue(['/home/u/photo.png']);
    const onNext = vi.fn();
    const user = userEvent.setup();
    render(<Step1Files onNext={onNext} />);

    await user.click(screen.getByRole('button', { name: /add files/i }));

    // The picked file surfaces in the list and the summary line.
    expect(await screen.findByText('photo.png')).toBeInTheDocument();
    expect(screen.getByText(/1 file selected/i)).toBeInTheDocument();

    await user.type(screen.getByPlaceholderText('My Encrypted Files'), 'My Vault');
    await user.click(screen.getByRole('button', { name: /next: security settings/i }));

    expect(onNext).toHaveBeenCalledTimes(1);
    expect(onNext).toHaveBeenCalledWith({
      name: 'My Vault',
      files: [{ path: '/home/u/photo.png', name: 'photo.png', mime: 'image/png', size: 2048 }],
    });
  });

  it('removes a file from the list when its remove control is clicked', async () => {
    open.mockResolvedValue(['/home/u/doc.pdf']);
    const user = userEvent.setup();
    render(<Step1Files onNext={vi.fn()} />);

    await user.click(screen.getByRole('button', { name: /add files/i }));
    expect(await screen.findByText('doc.pdf')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /remove doc\.pdf/i }));
    expect(screen.queryByText('doc.pdf')).not.toBeInTheDocument();
  });

  it('blocks advancing with no files selected', async () => {
    const onNext = vi.fn();
    const user = userEvent.setup();
    render(<Step1Files onNext={onNext} />);

    await user.type(screen.getByPlaceholderText('My Encrypted Files'), 'Empty Vault');
    await user.click(screen.getByRole('button', { name: /next: security settings/i }));

    expect(onNext).not.toHaveBeenCalled();
    expect(screen.getByText(/please select at least one file/i)).toBeInTheDocument();
  });
});
