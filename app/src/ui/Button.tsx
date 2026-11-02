import type { ButtonHTMLAttributes, ReactNode } from 'react'
import { GameIcon, type IconSource } from './GameIcon'

/**
 * Pill button.
 * - primary: warm-white pill, one main action per screen
 * - secondary: outlined pill (like "Подробнее" in the game)
 * - ghost: text only, for minor actions
 */
export function Button({
  variant = 'secondary',
  size = 'md',
  icon,
  children,
  className,
  ...rest
}: {
  variant?: 'primary' | 'secondary' | 'ghost'
  size?: 'sm' | 'md'
  icon?: IconSource
  children?: ReactNode
} & ButtonHTMLAttributes<HTMLButtonElement>) {
  const cls = ['ui-button', `ui-button--${variant}`, size === 'sm' && 'ui-button--sm', className].filter(Boolean).join(' ')
  return (
    <button type="button" className={cls} {...rest}>
      {icon && <GameIcon icon={icon} size={size === 'sm' ? 14 : 16} />}
      {children}
    </button>
  )
}
