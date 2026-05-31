import { useState, useEffect, useCallback, useRef } from 'react';

interface AutoLockOptions {
  timeout: number | null; // null means never
}

const DEFAULT_AUTO_LOCK: AutoLockOptions = {
  timeout: 5 * 60 * 1000, // 5 minutes default
};

export const useAutoLock = (onLock: () => void) => {
  const [options, setOptions] = useState<AutoLockOptions>(DEFAULT_AUTO_LOCK);
  const [lastActivity, setLastActivity] = useState(Date.now());
  const [isLocked, setIsLocked] = useState(false);
  const isLockedRef = useRef(isLocked);
  useEffect(() => { isLockedRef.current = isLocked; });

  // Load settings from localStorage
  useEffect(() => {
    const saved = localStorage.getItem('cryptainer_settings');
    if (saved) {
      try {
        const settings = JSON.parse(saved);
        if (settings.autoLockTimeout !== undefined) {
          setOptions({ timeout: settings.autoLockTimeout });
        }
      } catch {
        // Ignore parse errors
      }
    }
  }, []);

  // Track user activity
  const updateActivity = useCallback(() => {
    setLastActivity(Date.now());
    if (isLockedRef.current) {
      setIsLocked(false);
    }
  }, []);

  useEffect(() => {
    const events = ['mousedown', 'keydown', 'touchstart', 'scroll'];
    
    events.forEach(event => {
      document.addEventListener(event, updateActivity);
    });

    return () => {
      events.forEach(event => {
        document.removeEventListener(event, updateActivity);
      });
    };
  }, [updateActivity]);

  // Check for timeout
  useEffect(() => {
    if (options.timeout === null || isLocked) return;

    const interval = setInterval(() => {
      const elapsed = Date.now() - lastActivity;
      if (elapsed >= options.timeout!) {
        setIsLocked(true);
        onLock();
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [options.timeout, lastActivity, isLocked, onLock]);

  const unlock = useCallback(() => {
    setIsLocked(false);
    setLastActivity(Date.now());
  }, []);

  const updateTimeout = useCallback((timeout: number | null) => {
    setOptions({ timeout });
    setLastActivity(Date.now());
  }, []);

  return {
    isLocked,
    timeout: options.timeout,
    unlock,
    updateTimeout,
  };
};
