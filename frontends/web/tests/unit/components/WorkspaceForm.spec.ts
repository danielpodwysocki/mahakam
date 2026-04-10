import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import WorkspaceForm from '../../../components/WorkspaceForm.vue'

describe('WorkspaceForm', () => {
  it('renders name input and url input', () => {
    const wrapper = mount(WorkspaceForm, {
      props: { onSubmit: vi.fn(), submitting: false },
    })
    expect(wrapper.find('#env-name').exists()).toBe(true)
    expect(wrapper.find('input[type="url"]').exists()).toBe(true)
  })

  it('calls onSubmit with valid data including selected viewers', async () => {
    const onSubmit = vi.fn()
    const wrapper = mount(WorkspaceForm, {
      props: { onSubmit, submitting: false },
    })
    await wrapper.find('#env-name').setValue('my-ws')
    await wrapper.find('input[type="url"]').setValue('https://github.com/foo/bar')
    await wrapper.find('form').trigger('submit')
    expect(onSubmit).toHaveBeenCalledWith({
      name: 'my-ws',
      repos: ['https://github.com/foo/bar'],
      viewers: ['terminal'],
    })
  })

  it('shows error for invalid name', async () => {
    const onSubmit = vi.fn()
    const wrapper = mount(WorkspaceForm, {
      props: { onSubmit, submitting: false },
    })
    await wrapper.find('#env-name').setValue('Invalid Name!')
    await wrapper.find('input[type="url"]').setValue('https://github.com/foo/bar')
    await wrapper.find('form').trigger('submit')
    expect(onSubmit).not.toHaveBeenCalled()
    expect(wrapper.find('.error').exists()).toBe(true)
  })

  it('shows error when repos is empty', async () => {
    const onSubmit = vi.fn()
    const wrapper = mount(WorkspaceForm, {
      props: { onSubmit, submitting: false },
    })
    await wrapper.find('#env-name').setValue('my-ws')
    // Leave repos empty
    await wrapper.find('form').trigger('submit')
    expect(onSubmit).not.toHaveBeenCalled()
  })

  it('shows submitting state', () => {
    const wrapper = mount(WorkspaceForm, {
      props: { onSubmit: vi.fn(), submitting: true },
    })
    const submitBtn = wrapper.find('button[type="submit"]')
    expect(submitBtn.text()).toBe('Creating...')
    expect((submitBtn.element as HTMLButtonElement).disabled).toBe(true)
  })

  it('adds a second repo field when Add Repository clicked', async () => {
    const wrapper = mount(WorkspaceForm, {
      props: { onSubmit: vi.fn(), submitting: false },
    })
    const addBtn = wrapper.findAll('button[type="button"]').find((b) =>
      b.text().includes('Add Repository'),
    )!
    await addBtn.trigger('click')
    expect(wrapper.findAll('input[type="url"]')).toHaveLength(2)
  })

  it('removes a repo field when Remove clicked', async () => {
    const wrapper = mount(WorkspaceForm, {
      props: { onSubmit: vi.fn(), submitting: false },
    })
    // Add a second repo first
    const addBtn = wrapper.findAll('button[type="button"]').find((b) =>
      b.text().includes('Add Repository'),
    )!
    await addBtn.trigger('click')
    expect(wrapper.findAll('input[type="url"]')).toHaveLength(2)

    // Now remove the first one
    const removeBtns = wrapper
      .findAll('button[type="button"]')
      .filter((b) => b.text().includes('Remove'))
    await removeBtns[0].trigger('click')
    expect(wrapper.findAll('input[type="url"]')).toHaveLength(1)
  })

  it('renders viewer checkboxes for terminal, browser, and android', () => {
    const wrapper = mount(WorkspaceForm, {
      props: { onSubmit: vi.fn(), submitting: false },
    })
    const checkboxes = wrapper.findAll('input[type="checkbox"]')
    expect(checkboxes).toHaveLength(3)
  })

  it('terminal viewer is checked by default', () => {
    const wrapper = mount(WorkspaceForm, {
      props: { onSubmit: vi.fn(), submitting: false },
    })
    const checkboxes = wrapper.findAll('input[type="checkbox"]')
    const terminalCheckbox = checkboxes.find((c) => (c.element as HTMLInputElement).value === 'terminal')
    expect((terminalCheckbox!.element as HTMLInputElement).checked).toBe(true)
  })

  it('includes browser viewer when its checkbox is checked', async () => {
    const onSubmit = vi.fn()
    const wrapper = mount(WorkspaceForm, {
      props: { onSubmit, submitting: false },
    })
    await wrapper.find('#env-name').setValue('my-ws')
    await wrapper.find('input[type="url"]').setValue('https://github.com/foo/bar')

    const browserCheckbox = wrapper
      .findAll('input[type="checkbox"]')
      .find((c) => (c.element as HTMLInputElement).value === 'browser')!
    await browserCheckbox.trigger('change')
    ;(browserCheckbox.element as HTMLInputElement).checked = true
    await browserCheckbox.trigger('change')

    await wrapper.find('form').trigger('submit')
    const call = onSubmit.mock.calls[0][0]
    expect(call.viewers).toContain('browser')
  })
})
