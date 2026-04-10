import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import TerminalViewer from '../../../components/TerminalViewer.vue'

describe('TerminalViewer (generic viewer modal)', () => {
  it('renders an iframe with the provided src', () => {
    const wrapper = mount(TerminalViewer, {
      props: { title: 'my-env', src: '/projects/viewers/my-env/terminal/' },
    })
    const iframe = wrapper.find('iframe')
    expect(iframe.exists()).toBe(true)
    expect(iframe.attributes('src')).toBe('/projects/viewers/my-env/terminal/')
  })

  it('includes the title in the iframe title attribute', () => {
    const wrapper = mount(TerminalViewer, {
      props: { title: 'staging', src: '/projects/viewers/staging/terminal/' },
    })
    expect(wrapper.find('iframe').attributes('title')).toContain('staging')
  })

  it('displays the title in the header', () => {
    const wrapper = mount(TerminalViewer, {
      props: { title: 'prod-cluster', src: '/projects/viewers/prod-cluster/terminal/' },
    })
    expect(wrapper.text()).toContain('prod-cluster')
  })

  it('emits close when the dismiss button is clicked', async () => {
    const wrapper = mount(TerminalViewer, {
      props: { title: 'my-env', src: '/projects/viewers/my-env/terminal/' },
    })
    await wrapper.find('button').trigger('click')
    expect(wrapper.emitted('close')).toBeTruthy()
  })

  it('emits close when the backdrop is clicked', async () => {
    const wrapper = mount(TerminalViewer, {
      props: { title: 'my-env', src: '/projects/viewers/my-env/terminal/' },
      attachTo: document.body,
    })
    await wrapper.find('.console-backdrop').trigger('click')
    expect(wrapper.emitted('close')).toBeTruthy()
    wrapper.unmount()
  })
})
