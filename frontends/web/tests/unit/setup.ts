import { config } from '@vue/test-utils'

// Stub NuxtLink as a plain anchor so unit tests can inspect href and text.
config.global.stubs = {
  NuxtLink: {
    props: ['to'],
    template: '<a :href="to" v-bind="$attrs"><slot /></a>',
  },
}
