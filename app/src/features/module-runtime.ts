import type { ModuleContext, ModuleDef, ModuleSettings, StopModule } from './types'

interface Running { controller: AbortController; stop?: StopModule; settings: ModuleSettings | undefined }
/**
 * Runs modules' background work (`start` from ui.tsx) while they are enabled.
 * A disabled module, or one whose settings changed, is aborted and stopped
 * at once; a failing module never stops the shell or the others.
 */
export class ModuleRuntime {
  private running = new Map<string, Running>()
  private context: (module: ModuleDef) => Omit<ModuleContext, 'signal' | 'settings'>
  private onError: (message: string) => void
  constructor(
    context: (module: ModuleDef) => Omit<ModuleContext, 'signal' | 'settings'>,
    onError: (message: string) => void,
  ) {
    this.context = context
    this.onError = onError
  }

  update(modules: readonly ModuleDef[], settings: Record<string, ModuleSettings> = {}): void {
    for (const [id, running] of this.running) {
      if (!modules.some((m) => m.id === id) || running.settings !== settings[id]) {
        running.controller.abort()
        try { running.stop?.() } catch (error) { this.onError(`Не удалось остановить модуль: ${String(error)}`) }
        this.running.delete(id)
      }
    }
    for (const module of modules) {
      if (this.running.has(module.id)) continue
      const controller = new AbortController()
      try {
        const stop = module.start?.({ ...this.context(module), signal: controller.signal,
          settings: { ...module.settings?.defaults, ...settings[module.id] } })
        this.running.set(module.id, { controller, stop: stop || undefined, settings: settings[module.id] })
      } catch (error) {
        controller.abort()
        this.onError(`Модуль «${module.title}» не запустился: ${String(error)}`)
      }
    }
  }
}
