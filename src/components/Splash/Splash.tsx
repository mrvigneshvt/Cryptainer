import { useEffect, useState } from 'react';
import logoUrl from '../../assets/cryptainer-logo.png';
import './Splash.css';

interface SplashProps {
  /** Minimum time the splash stays visible, in ms. */
  duration?: number;
}

/**
 * Full-screen branded splash shown once on app launch, then fades out.
 * Portable across desktop and the Android webview (pure DOM, no native window).
 */
export const Splash: React.FC<SplashProps> = ({ duration = 1400 }) => {
  const [leaving, setLeaving] = useState(false);
  const [done, setDone] = useState(false);

  useEffect(() => {
    const fadeAt = setTimeout(() => setLeaving(true), duration);
    const removeAt = setTimeout(() => setDone(true), duration + 400);
    return () => {
      clearTimeout(fadeAt);
      clearTimeout(removeAt);
    };
  }, [duration]);

  if (done) return null;

  return (
    <div className={`splash ${leaving ? 'splash-leaving' : ''}`} role="status" aria-label="Loading Cryptainer">
      <div className="splash-inner">
        <img className="splash-logo" src={logoUrl} alt="Cryptainer" />
        <span className="splash-title">Cryptainer</span>
        <div className="splash-spinner" />
      </div>
    </div>
  );
};
