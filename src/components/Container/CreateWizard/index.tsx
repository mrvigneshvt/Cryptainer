import React, { useState } from 'react';
import { Modal, ProgressOverlay } from '../../UI';
import { Step1Files } from './Step1Files';
import { Step2Config } from './Step2Config';
import { useVaultStore } from '../../../store/vaultStore';
import { useTauriProgress } from '../../../hooks/useTauriProgress';
import type { FileInput, KdfParams } from '../../../types/vault';
import './index.css';

interface CreateWizardProps {
  onClose: () => void;
}

export const CreateWizard: React.FC<CreateWizardProps> = ({ onClose }) => {
  const [step, setStep] = useState<1 | 2>(1);
  const [step1Data, setStep1Data] = useState<{ name: string; files: File[] } | null>(null);
  const { createContainer } = useVaultStore();
  const [isLoading, setIsLoading] = useState(false);
  const { progress } = useTauriProgress(isLoading);

  const handleStep1Next = (data: { name: string; files: File[] }) => {
    setStep1Data(data);
    setStep(2);
  };

  const handleStep2Back = () => setStep(1);

  const handleCreate = async (config: {
    password: string;
    kdfParams: KdfParams;
    hint?: string;
  }) => {
    if (!step1Data) return;
    setIsLoading(true);
    try {
      const fileInputs: FileInput[] = await Promise.all(
        step1Data.files.map(async (file) => {
          const arrayBuffer = await file.arrayBuffer();
          return {
            name: file.name,
            mime: file.type || 'application/octet-stream',
            data: Array.from(new Uint8Array(arrayBuffer)),
          };
        })
      );
      await createContainer({
        name: step1Data.name,
        kdf_params: config.kdfParams,
        hint: config.hint,
        password: config.password,
        files: fileInputs,
      });
      onClose();
    } catch (e) {
      console.error('Failed to create container:', e);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Modal open={true} onClose={onClose} size="md">
      <div className="create-wizard">
        {/* Step indicator */}
        <div className="wizard-steps">
          <div className={`wizard-step ${step === 1 ? 'active' : ''} ${step > 1 ? 'done' : ''}`}>
            <div className="wizard-step-circle">
              {step > 1 ? (
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              ) : (
                <span>1</span>
              )}
            </div>
            <span className="wizard-step-label">Files</span>
          </div>
          <div className="wizard-step-connector">
            <div className={`wizard-connector-fill ${step > 1 ? 'filled' : ''}`} />
          </div>
          <div className={`wizard-step ${step === 2 ? 'active' : ''}`}>
            <div className="wizard-step-circle">
              <span>2</span>
            </div>
            <span className="wizard-step-label">Security</span>
          </div>
        </div>

        {step === 1 ? (
          <Step1Files onNext={handleStep1Next} />
        ) : (
          <Step2Config
            onBack={handleStep2Back}
            onCreate={handleCreate}
            isLoading={isLoading}
          />
        )}
      </div>
      <ProgressOverlay open={isLoading} progress={progress} fallbackMessage="Encrypting files…" />
    </Modal>
  );
};
