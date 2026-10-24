import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import type { ImportPreview, ImportReport } from '@/services/types'
import ImportDialog from './ImportDialog.vue'
import { useUiStore } from '@/stores/ui'

const openDialog = vi.fn(async () => '/Users/me/Vault' as string | null)
const importScan = vi.fn<(path: string) => Promise<ImportPreview>>()
const importNotes = vi.fn<(path: string, options: { parentDirTag?: boolean }) => Promise<ImportReport>>()
const cancelImport = vi.fn(async () => {})
const deleteImportBatch = vi.fn<(tag: string) => Promise<number>>(async () => 3)
const listNotes = vi.fn(async () => ({ items: [], nextCursor: null }))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: (...args: unknown[]) => openDialog(...(args as [])),
  save: async () => null,
  message: async () => {},
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: async () => () => {},
}))

vi.mock('@/services/commands', () => ({
  isDesktop: true,
  CommandError: class extends Error {},
  errorMessage: (error: unknown) => String(error),
  api: {
    importScan: (...args: [string]) => importScan(...args),
    importNotes: (...args: [string, { parentDirTag?: boolean }]) => importNotes(...args),
    cancelImport: () => cancelImport(),
    deleteImportBatch: (...args: [string]) => deleteImportBatch(...args),
    listNotes: (...args: unknown[]) => listNotes(...(args as [])),
    listDeletedNotes: async () => [],
    listTags: async () => [],
    getNote: async () => null,
  },
}))

const preview: ImportPreview = {
  root: '/Users/me/Vault',
  totalFiles: 5,
  toImport: 3,
  skippedExisting: 2,
  hiddenSkipped: 4,
  localFileRefs: 2,
  unreadable: [],
}

function report(overrides: Partial<ImportReport> = {}): ImportReport {
  return {
    imported: 3,
    skippedExisting: 2,
    failed: [],
    hiddenSkipped: 4,
    decodedAsGb18030: 0,
    localFileReferences: 2,
    cancelled: false,
    elapsedMs: 1450,
    batchTag: 'import-2026-02-20',
    totalFiles: 5,
    ...overrides,
  }
}

async function mountDialog() {
  const wrapper = mount(ImportDialog, { attachTo: document.body })
  useUiStore().importOpen = true
  await flushPromises()
  return wrapper
}

async function chooseAndScan(wrapper: Awaited<ReturnType<typeof mountDialog>>) {
  await wrapper.findAll('button').find((button) => button.text().includes('选择目录'))!.trigger('click')
  await flushPromises()
}

beforeEach(() => {
  setActivePinia(createPinia())
  openDialog.mockClear()
  importScan.mockReset()
  importNotes.mockReset()
  cancelImport.mockClear()
  deleteImportBatch.mockClear()
  listNotes.mockClear()
  importScan.mockResolvedValue(preview)
  importNotes.mockResolvedValue(report())
})

describe('import dialog stages', () => {
  it('goes pick → scan → import → report and refreshes the list', async () => {
    const wrapper = await mountDialog()
    expect(wrapper.text()).toContain('尚未选择')

    await chooseAndScan(wrapper)
    expect(openDialog).toHaveBeenCalledWith(
      expect.objectContaining({ directory: true, multiple: false }),
    )
    expect(importScan).toHaveBeenCalledWith('/Users/me/Vault')
    expect(wrapper.text()).toContain('待导入')
    expect(wrapper.text()).toContain('跳过（已导入）')

    const start = wrapper.findAll('button').find((button) => button.text() === '开始导入')!
    await start.trigger('click')
    await flushPromises()

    expect(importNotes).toHaveBeenCalledWith('/Users/me/Vault', { parentDirTag: false })
    expect(wrapper.text()).toContain('导入完成')
    expect(wrapper.text()).toContain('1.4 秒')
    expect(listNotes).toHaveBeenCalled()

    // A clean run renders no failure section at all.
    expect(wrapper.find('.failures').exists()).toBe(false)

    // Spec "完成后可返回": closing the report also closes the modal.
    await wrapper.find('button.close').trigger('click')
    expect(useUiStore().importOpen).toBe(false)
    expect(wrapper.find('[role="dialog"]').exists()).toBe(false)

    // Reopening starts from the pick stage, not the previous report.
    useUiStore().importOpen = true
    await flushPromises()
    expect(wrapper.text()).toContain('尚未选择')
    expect(wrapper.find('.report').exists()).toBe(false)
  })

  it('refuses to close while the import is running and offers cancel', async () => {
    let resolveImport: (value: ImportReport) => void = () => {}
    importNotes.mockReturnValue(
      new Promise<ImportReport>((resolve) => {
        resolveImport = resolve
      }),
    )

    const wrapper = await mountDialog()
    await chooseAndScan(wrapper)
    await wrapper.findAll('button').find((button) => button.text() === '开始导入')!.trigger('click')
    await flushPromises()

    // No close affordance during the run, and clicking the overlay is refused.
    expect(wrapper.find('button.close').exists()).toBe(false)
    await wrapper.find('.overlay').trigger('click')
    expect(wrapper.text()).toContain('导入进行中无法关闭')
    expect(wrapper.find('[role="dialog"]').exists()).toBe(true)

    const cancel = wrapper.findAll('button').find((button) => button.text().includes('取消导入'))!
    await cancel.trigger('click')
    await flushPromises()
    expect(cancelImport).toHaveBeenCalled()

    resolveImport(report())
    await flushPromises()
    expect(wrapper.text()).toContain('导入完成')
    expect(wrapper.find('button.close').exists()).toBe(true)
  })

  it('passes the parent-directory-tag option through', async () => {
    const wrapper = await mountDialog()
    await chooseAndScan(wrapper)
    await wrapper.find('input[type="checkbox"]').setValue(true)
    await wrapper.findAll('button').find((button) => button.text() === '开始导入')!.trigger('click')
    await flushPromises()

    expect(importNotes).toHaveBeenCalledWith('/Users/me/Vault', { parentDirTag: true })
  })

  it('lists every failure with its reason when some files fail', async () => {
    importNotes.mockResolvedValue(
      report({
        imported: 1,
        skippedExisting: 0,
        failed: [
          { path: '/Users/me/Vault/broken.md', reason: '无法按 UTF-8 或 GB18030 解码' },
          { path: '/Users/me/Vault/locked.md', reason: '读取失败: permission denied' },
        ],
      }),
    )

    const wrapper = await mountDialog()
    await chooseAndScan(wrapper)
    await wrapper.findAll('button').find((button) => button.text() === '开始导入')!.trigger('click')
    await flushPromises()

    expect(wrapper.find('.failures').exists()).toBe(true)
    expect(wrapper.findAll('.failures li')).toHaveLength(2)
    expect(wrapper.text()).toContain('/Users/me/Vault/broken.md')
    expect(wrapper.text()).toContain('无法按 UTF-8 或 GB18030 解码')
    expect(wrapper.find('[data-testid="failed-count"]').text()).toBe('2')
  })

  it('deletes the batch and reports the trash result', async () => {
    const wrapper = await mountDialog()
    await chooseAndScan(wrapper)
    await wrapper.findAll('button').find((button) => button.text() === '开始导入')!.trigger('click')
    await flushPromises()

    const undo = wrapper.findAll('button').find((button) => button.text() === '删除本批')!
    await undo.trigger('click')
    await flushPromises()

    expect(deleteImportBatch).toHaveBeenCalledWith('import-2026-02-20')
    expect(wrapper.text()).toContain('已把 3 条笔记移入回收站')
    expect(wrapper.findAll('button').some((button) => button.text() === '删除本批')).toBe(false)
  })
})
