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
}

describe('EnvironmentList', () => {
  it('renders environment data in table', () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [mockEnv], onDelete: vi.fn() },
    })
    expect(wrapper.find('table').exists()).toBe(true)
    expect(wrapper.text()).toContain('test-env')
    expect(wrapper.text()).toContain('env-test-env')
    expect(wrapper.text()).toContain('pending')
  })

  it('calls onDelete with the environment name when Delete clicked', async () => {
    const onDelete = vi.fn()
    const wrapper = mount(EnvironmentList, {
      props: { environments: [mockEnv], onDelete },
    })
    await wrapper.find('button').trigger('click')
    expect(onDelete).toHaveBeenCalledWith('test-env')
  })

  it('renders only the header row for an empty list', () => {
    const wrapper = mount(EnvironmentList, {
      props: { environments: [], onDelete: vi.fn() },
    })
    // tbody should have no rows
    expect(wrapper.findAll('tbody tr')).toHaveLength(0)
  })

  it('joins multiple repos with a comma', () => {
    const env: Environment = { ...mockEnv, repos: ['https://a.com', 'https://b.com'] }
    const wrapper = mount(EnvironmentList, {
      props: { environments: [env], onDelete: vi.fn() },
    })
    expect(wrapper.text()).toContain('https://a.com, https://b.com')
  })
})
