import React, { useMemo } from 'react';
import { Highlight, themes } from 'prism-react-renderer';
import './TextPreview.css';

interface TextPreviewProps {
  data: Uint8Array;
  name: string;
}

const CODE_EXTENSIONS: Record<string, string> = {
  'rs': 'rust',
  'ts': 'typescript',
  'tsx': 'tsx',
  'js': 'javascript',
  'jsx': 'jsx',
  'py': 'python',
  'go': 'go',
  'java': 'java',
  'cpp': 'cpp',
  'c': 'c',
  'h': 'c',
  'cs': 'csharp',
  'rb': 'ruby',
  'php': 'php',
  'swift': 'swift',
  'kt': 'kotlin',
  'scala': 'scala',
  'r': 'r',
  'sql': 'sql',
  'sh': 'bash',
  'bash': 'bash',
  'zsh': 'bash',
  'yaml': 'yaml',
  'yml': 'yaml',
  'json': 'json',
  'xml': 'xml',
  'html': 'html',
  'css': 'css',
  'scss': 'scss',
  'sass': 'sass',
  'less': 'less',
  'md': 'markdown',
  'markdown': 'markdown',
};

export const TextPreview: React.FC<TextPreviewProps> = ({ data, name }) => {
  const { text, language } = useMemo(() => {
    const decoder = new TextDecoder('utf-8', { fatal: true });
    const text = decoder.decode(data);
    
    const ext = name.split('.').pop()?.toLowerCase() || '';
    const lang = CODE_EXTENSIONS[ext] || 'text';
    
    return { text, language: lang };
  }, [data, name]);

  if (language === 'text') {
    return (
      <div className="text-preview">
        <pre className="plain-text">{text}</pre>
      </div>
    );
  }

  return (
    <div className="text-preview code-preview">
      <Highlight theme={themes.vsDark} code={text} language={language}>
        {({ className, style, tokens, getLineProps, getTokenProps }) => (
          <pre className={className} style={style}>
            {tokens.map((line, i) => (
              <div key={i} {...getLineProps({ line })}>
                <span className="line-number">{i + 1}</span>
                {line.map((token, key) => (
                  <span key={key} {...getTokenProps({ token })} />
                ))}
              </div>
            ))}
          </pre>
        )}
      </Highlight>
    </div>
  );
};
