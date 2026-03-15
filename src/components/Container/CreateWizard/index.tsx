import React, { useState } from 'react';
import { Modal } from '../../UI';
import { Step1Files } from './Step1Files';
import { Step2Config } from './Step2Config';
import { useVaultStore } from '../../../store/vaultStore';
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

  const handleStep1Next = (data: { name: string; files: File[] }) => {
    setStep1Data(data);
    setStep(2);
  };

  const handleStep2Back = () => {
    setStep(1);
  };

  const handleCreate = async (config: {
    password: string;
    kdfParams: KdfParams;
    hint?: string;
  }) => {
    if (!step1Data) return;

    setIsLoading(true);
    try {
      // Convert Files to FileInput
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
    </Modal>
  );
};
