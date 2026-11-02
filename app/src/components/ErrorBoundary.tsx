import { Component, type ReactNode } from 'react'
import { Button, EmptyState, Page } from '../ui'

/**
 * Wraps one screen: if it throws while rendering, only that screen shows an
 * error, the shell (menu, profile, other screens) keeps working.
 */
export class ErrorBoundary extends Component<{ name: string; children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null }

  static getDerivedStateFromError(error: Error) {
    return { error }
  }

  componentDidCatch(error: Error) {
    console.error(`[${this.props.name}] screen crashed`, error)
  }

  render() {
    if (!this.state.error) return this.props.children
    return (
      <Page title="Экран не открылся">
        <EmptyState icon="settings" title="Что-то пошло не так">
          В экране «{this.props.name}» произошла ошибка: {this.state.error.message}
          <div style={{ marginTop: 'var(--space-4)' }}>
            <Button onClick={() => this.setState({ error: null })}>Попробовать снова</Button>
          </div>
        </EmptyState>
      </Page>
    )
  }
}
