<template>
<div>
  <b-navbar
    toggleable="lg"
    type="light"
    variant="light"
    class="border-bottom emphasis"
  >
    <b-navbar-brand href="/">Syncabull</b-navbar-brand>
    <b-navbar-nav>
      <b-nav-item href="https://github.com/JosiahBull/syncabull/"><b-icon icon="github" font-scale="1.5" class="mr-1"></b-icon> Repository </b-nav-item>
    </b-navbar-nav>

    <b-navbar-nav class="ml-auto">
      <b-button size="lg" variant="primary" class="font-weight-bold mx-auto align-middle"
          href="https://github.com/JosiahBull/syncabull#installation">Start Syncing</b-button>
    </b-navbar-nav>
  </b-navbar>
</div>
</template>

<script>
export default {
  name: 'Nav',
}
</script>

<style scoped>

.emphasis {
  /* add some emphasis to the bottom of the navbar, so that it becomes distinct against the white-on-white background */
  box-shadow: 0 0.25rem 0.75rem rgba(0, 0, 0, 0.3);
  -webkit-box-shadow: 0 0.25rem 0.75rem rgba(0, 0, 0, 0.3);
  -moz-box-shadow: 0 0.25rem 0.75rem rgba(0, 0, 0, 0.3);
}
</style>
