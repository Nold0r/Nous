import type { ModuleContext, ModuleDef, ModuleSettings, ServiceId, StopModule } from './types'

interface Running { controller: AbortController; stop?: StopModule; settings: ModuleSettings | undefined }
/** Serializes service transitions; disabled modules are aborted immediately. */
export class ModuleRuntime {
  private running = new Map<string, Running>()
  private desired: readonly ModuleDef[] = []
  private settings: Record<string, ModuleSettings> = {}
  private queue: Promise<void> = Promise.resolve()
  private generation = 0

  private services: string | null = null
  private setServices: (services: readonly ServiceId[]) => Promise<void>
  private context: (module: ModuleDef) => Omit<ModuleContext, 'signal' | 'settings'>
  private onError: (message: string) => void
  constructor(
    setServices: (services: readonly ServiceId[]) => Promise<void>,
    context: (module: ModuleDef) => Omit<ModuleContext, 'signal' | 'settings'>,
    onError: (message: string) => void,
  ) {
    this.setServices = setServices
    this.context = context
    this.onError = onError
  }

  update(modules: readonly ModuleDef[], settings: Record<string, ModuleSettings> = {}): Promise<void> {
    this.desired = modules
    this.settings = settings
    const generation = ++this.generation
    for (const [id, running] of this.running) {
      if (!modules.some((m) => m.id === id) || running.settings !== settings[id]) {
        running.controller.abort()
        try { running.stop?.() } catch (error) { this.onError(`Не удалось остановить модуль: ${String(error)}`) }
        this.running.delete(id)
      }
    }
    this.queue = this.queue.then(async () => {
      if (generation !== this.generation) return
      const required = [...new Set(this.desired.flatMap((m) => m.requires ?? []))]
      const key = required.sort().join(',')
      if (this.services !== key) {
        await this.setServices(required)
        this.services = key
      }
      if (generation !== this.generation) return
      for (const module of this.desired) {
        if (this.running.has(module.id)) continue
        const controller = new AbortController()
        try {
          const stop = module.start?.({ ...this.context(module), signal: controller.signal,
            settings: { ...module.settings?.defaults, ...this.settings[module.id] } })
          this.running.set(module.id, { controller, stop: stop || undefined, settings: this.settings[module.id] })
        } catch (error) {
          controller.abort()
          this.onError(`Модуль «${module.title}» не запустился: ${String(error)}`)
        }
      }
    }).catch((error: unknown) => this.onError(`Не удалось подключить сервисы модулей: ${String(error)}`))
    return this.queue
  }
}
