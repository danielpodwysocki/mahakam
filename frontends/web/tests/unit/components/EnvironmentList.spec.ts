import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import EnvironmentList from '../../../components/EnvironmentList.vue'
import type { Environment } from '../../../api/environments'

const mockEnv: Environment = {
  id: '550e8400-e29b-41d4-a716-446655440000',
  name: 'test-env',
  repos: ['https://github.com/foo/bar'],
  namespace: 'env-test-env',
  status: 'pending',
  created_at: '2024-01-01T00:00:00Z',
  viewers: [],
}

const readyEnvWithViewers: Environment = {
  ...mockEnv,
  status: 'ready',
  viewers: [
    { name: 'terminal', display_name: 'Terminal', path: '/projects/viewers/test-env/terminal' },
    { name: 'browser', display_name: 'Browser', path: '/projects/viewers/test-env/browser' },
  ],
}

describe('EnvironmentList', () => {
  it('renders environment data in table', () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [mockEnv], onDelete: vi.fn() },
    })
    expect(wrapper.find('table').exists()).toBe(true)
    expect(wrapper.text()).toContain('test-env')
    expect(wrapper.text()).toContain('env-test-env')
  })

  it('shows "connecting" label for pending status', () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [mockEnv], onDelete: vi.fn() },
    })
    expect(wrapper.text()).toContain('connecting')
    expect(wrapper.text()).not.toContain('pending')
  })

  it('shows "connecting" label for provisioning status', () => {
    const env: Environment = { ...mockEnv, status: 'provisioning' }
    const wrapper = mount(EnvironmentList, {
      props: { environments: [env], onDelete: vi.fn() },
    })
    expect(wrapper.text()).toContain('connecting')
  })

  it('shows "ready" label for ready status', () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [readyEnvWithViewers], onDelete: vi.fn() },
    })
    expect(wrapper.text()).toContain('ready')
  })

  it('disables delete button when env is not settled', () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [mockEnv], onDelete: vi.fn() },
    })
    expect(wrapper.find('button').attributes('disabled')).toBeDefined()
  })

  it('enables delete button when env is ready', () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [readyEnvWithViewers], onDelete: vi.fn() },
    })
    expect(wrapper.find('button').attributes('disabled')).toBeUndefined()
  })

  it('calls onDelete with the environment name when Delete clicked on settled env', async () => {
    const onDelete = vi.fn()
    const wrapper = mount(EnvironmentList, {
      props: { environments: [readyEnvWithViewers], onDelete },
    })
    await wrapper.find('button').trigger('click')
    expect(onDelete).toHaveBeenCalledWith('test-env')
  })

  it('renders only the header row for an empty list', () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [], onDelete: vi.fn() },
    })
    expect(wrapper.findAll('tbody tr')).toHaveLength(0)
  })

  it('joins multiple repos with a comma', () => {
    const env: Environment = { ...mockEnv, repos: ['https://a.com', 'https://b.com'] }
    const wrapper = mount(EnvironmentList, {
      props: { environments: [env], onDelete: vi.fn() },
    })
    expect(wrapper.text()).toContain('https://a.com, https://b.com')
  })

  it('shows no viewer buttons when env has no viewers', () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [mockEnv], onDelete: vi.fn() },
    })
    const viewerBtns = wrapper.findAll('button').filter((b) => b.classes('btn-viewer'))
    expect(viewerBtns).toHaveLength(0)
  })

  it('renders a button for each viewer', () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [readyEnvWithViewers], onDelete: vi.fn() },
    })
    const viewerBtns = wrapper.findAll('button').filter((b) => b.classes('btn-viewer'))
    expect(viewerBtns).toHaveLength(2)
    expect(viewerBtns[0].text()).toBe('Terminal')
    expect(viewerBtns[1].text()).toBe('Browser')
  })

  it('disables viewer buttons when env is not ready', () => {
    const env: Environment = {
      ...mockEnv,
      status: 'failed',
      viewers: [{ name: 'terminal', display_name: 'Terminal', path: '/projects/viewers/test-env/terminal' }],
    }
    const wrapper = mount(EnvironmentList, {
      props: { environments: [env], onDelete: vi.fn() },
    })
    const viewerBtn = wrapper.findAll('button').find((b) => b.classes('btn-viewer'))
    expect(viewerBtn!.attributes('disabled')).toBeDefined()
  })

  it('opens viewer modal with correct src when viewer button clicked', async () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [readyEnvWithViewers], onDelete: vi.fn() },
    })
    const terminalBtn = wrapper.findAll('button').find((b) => b.text() === 'Terminal')!
    await terminalBtn.trigger('click')
    expect(wrapper.find('iframe').exists()).toBe(true)
    expect(wrapper.find('iframe').attributes('src')).toBe('/projects/viewers/test-env/terminal/')
  })

  it('hides viewer modal after dismiss', async () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [readyEnvWithViewers], onDelete: vi.fn() },
      attachTo: document.body,
    })
    const terminalBtn = wrapper.findAll('button').find((b) => b.text() === 'Terminal')!
    await terminalBtn.trigger('click')
    expect(wrapper.find('iframe').exists()).toBe(true)
    const dismissBtn = wrapper.findAll('button').find((b) => b.text().includes('Dismiss'))
    await dismissBtn!.trigger('click')
    expect(wrapper.find('iframe').exists()).toBe(false)
    wrapper.unmount()
  })
})
