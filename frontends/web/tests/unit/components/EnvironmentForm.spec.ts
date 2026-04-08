import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import EnvironmentForm from '../../../components/EnvironmentForm.vue'

describe('EnvironmentForm', () => {
  it('renders name input and url input', () => {
    const wrapper = mount(EnvironmentForm, {
      props: { onSubmit: vi.fn(), submitting: false },
    })
    expect(wrapper.find('#env-name').exists()).toBe(true)
    expect(wrapper.find('input[type="url"]').exists()).toBe(true)
  })

  it('calls onSubmit with valid data', async () => {
    const onSubmit = vi.fn()
    const wrapper = mount(EnvironmentForm, {
      props: { onSubmit, submitting: false },
    })
    await wrapper.find('#env-name').setValue('my-env')
    await wrapper.find('input[type="url"]').setValue('https://github.com/foo/bar')
    await wrapper.find('form').trigger('submit')
    expect(onSubmit).toHaveBeenCalledWith({
      name: 'my-env',
      repos: ['https://github.com/foo/bar'],
    })
  })

  it('shows error for invalid name', async () => {
    const onSubmit = vi.fn()
    const wrapper = mount(EnvironmentForm, {
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
    const wrapper = mount(EnvironmentForm, {
      props: { onSubmit, submitting: false },
    })
    await wrapper.find('#env-name').setValue('my-env')
    // Leave repos empty
    await wrapper.find('form').trigger('submit')
    expect(onSubmit).not.toHaveBeenCalled()
  })

  it('shows submitting state', () => {
    const wrapper = mount(EnvironmentForm, {
      props: { onSubmit: vi.fn(), submitting: true },
    })
    const submitBtn = wrapper.find('button[type="submit"]')
    expect(submitBtn.text()).toBe('Creating...')
    expect((submitBtn.element as HTMLButtonElement).disabled).toBe(true)
  })

  it('adds a second repo field when Add Repository clicked', async () => {
    const wrapper = mount(EnvironmentForm, {
      props: { onSubmit: vi.fn(), submitting: false },
    })
    const addBtn = wrapper.findAll('button[type="button"]').find((b) =>
      b.text().includes('Add Repository'),
    )!
    await addBtn.trigger('click')
    expect(wrapper.findAll('input[type="url"]')).toHaveLength(2)
  })

  it('removes a repo field when Remove clicked', async () => {
    const wrapper = mount(EnvironmentForm, {
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
})
