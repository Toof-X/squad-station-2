import type { Theme } from '../hooks/useTheme';

interface ThemeToggleProps {
  theme: Theme;
  onToggle: () => void;
}

export function ThemeToggle({ theme, onToggle }: ThemeToggleProps) {
  const isDark = theme === 'dark';

  return (
    <button
      onClick={onToggle}
      aria-label="Toggle theme"
      className="p-2 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-400 transition-colors text-base leading-none"
    >
      {/* Sun icon when dark (clicking switches to light); Moon icon when light */}
      {isDark ? '\u2600' : '\u263D'}
    </button>
  );
}
