import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import WorkspaceList from '../../../components/WorkspaceList.vue'
import type { Workspace } from '../../../api/workspaces'

const mockWs: Workspace = {
  id: '550e8400-e29b-41d4-a716-446655440000',
  name: 'test-ws',
  repos: ['https://github.com/foo/bar'],
  namespace: 'ws-test-ws',
  status: 'pending',
  created_at: '2024-01-01T00:00:00Z',
  viewers: [],
  project: 'default',
}

const readyWs: Workspace = {
  ...mockWs,
  status: 'ready',
  viewers: [
    { name: 'terminal', display_name: 'Terminal', path: '/projects/viewers/test-ws/terminal' },
    { name: 'browser', display_name: 'Browser', path: '/projects/viewers/test-ws/browser' },
  ],
}

describe('WorkspaceList', () => {
  it('renders workspace data in table', () => {
    const wrapper = mount(WorkspaceList, {
      props: { workspaces: [mockWs], onDelete: vi.fn(), project: 'default' },
    })
    expect(wrapper.find('table').exists()).toBe(true)
    expect(wrapper.text()).toContain('test-ws')
    expect(wrapper.text()).toContain('ws-test-ws')
  })

  it('shows "connecting" label for pending status', () => {
    const wrapper = mount(WorkspaceList, {
      props: { workspaces: [mockWs], onDelete: vi.fn(), project: 'default' },
    })
    expect(wrapper.text()).toContain('connecting')
    expect(wrapper.text()).not.toContain('pending')
  })

  it('shows "connecting" label for provisioning status', () => {
    const ws: Workspace = { ...mockWs, status: 'provisioning' }
    const wrapper = mount(WorkspaceList, {
      props: { workspaces: [ws], onDelete: vi.fn(), project: 'default' },
    })
    expect(wrapper.text()).toContain('connecting')
  })

  it('shows "ready" label for ready status', () => {
    const wrapper = mount(WorkspaceList, {
      props: { workspaces: [readyWs], onDelete: vi.fn(), project: 'default' },
    })
    expect(wrapper.text()).toContain('ready')
  })

  it('disables delete button when workspace is not settled', () => {
    const wrapper = mount(WorkspaceList, {
      props: { workspaces: [mockWs], onDelete: vi.fn(), project: 'default' },
    })
    const deleteBtn = wrapper.findAll('button').find((b) => b.text() === 'Delete')
    expect(deleteBtn!.attributes('disabled')).toBeDefined()
  })

  it('enables delete button when workspace is ready', () => {
    const wrapper = mount(WorkspaceList, {
      props: { workspaces: [readyWs], onDelete: vi.fn(), project: 'default' },
    })
    const deleteBtn = wrapper.findAll('button').find((b) => b.text() === 'Delete')
    expect(deleteBtn!.attributes('disabled')).toBeUndefined()
  })

  it('calls onDelete with the workspace name when Delete clicked on settled workspace', async () => {
    const onDelete = vi.fn()
    const wrapper = mount(WorkspaceList, {
      props: { workspaces: [readyWs], onDelete, project: 'default' },
    })
    const deleteBtn = wrapper.findAll('button').find((b) => b.text() === 'Delete')!
    await deleteBtn.trigger('click')
    expect(onDelete).toHaveBeenCalledWith('test-ws')
  })

  it('renders only the header row for an empty list', () => {
    const wrapper = mount(WorkspaceList, {
      props: { workspaces: [], onDelete: vi.fn(), project: 'default' },
    })
    expect(wrapper.findAll('tbody tr')).toHaveLength(0)
  })

  it('joins multiple repos with a comma', () => {
    const ws: Workspace = { ...mockWs, repos: ['https://a.com', 'https://b.com'] }
    const wrapper = mount(WorkspaceList, {
      props: { workspaces: [ws], onDelete: vi.fn(), project: 'default' },
    })
    expect(wrapper.text()).toContain('https://a.com, https://b.com')
  })

  it('renders an Open link for each workspace', () => {
    const wrapper = mount(WorkspaceList, {
      props: { workspaces: [mockWs], onDelete: vi.fn(), project: 'default' },
    })
    const openLink = wrapper.find('a.btn-open')
    expect(openLink.exists()).toBe(true)
    expect(openLink.text()).toBe('Open')
    expect(openLink.attributes('href')).toContain('/projects/default/workspaces/test-ws')
  })
})
