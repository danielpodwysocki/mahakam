import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import TerminalViewer from '../../../components/TerminalViewer.vue'

describe('TerminalViewer', () => {
  it('renders an iframe with the correct src for the environment', () => {
    const wrapper = mount(TerminalViewer, {
      props: { envName: 'my-env' },
    })
    const iframe = wrapper.find('iframe')
    expect(iframe.exists()).toBe(true)
    expect(iframe.attributes('src')).toBe('/projects/viewers/my-env/')
  })

  it('includes the env name in the iframe title', () => {
    const wrapper = mount(TerminalViewer, {
      props: { envName: 'staging' },
    })
    expect(wrapper.find('iframe').attributes('title')).toContain('staging')
  })

  it('displays the env name in the header', () => {
    const wrapper = mount(TerminalViewer, {
      props: { envName: 'prod-cluster' },
    })
    expect(wrapper.text()).toContain('prod-cluster')
  })

  it('emits close when the dismiss button is clicked', async () => {
    const wrapper = mount(TerminalViewer, {
      props: { envName: 'my-env' },
    })
    await wrapper.find('button').trigger('click')
    expect(wrapper.emitted('close')).toBeTruthy()
  })

  it('emits close when the backdrop is clicked', async () => {
    const wrapper = mount(TerminalViewer, {
      props: { envName: 'my-env' },
      attachTo: document.body,
    })
    await wrapper.find('.console-backdrop').trigger('click')
    expect(wrapper.emitted('close')).toBeTruthy()
    wrapper.unmount()
  })
})
