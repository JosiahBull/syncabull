<template>
<div>
  <b-navbar toggleable="lg" type="white" variant="white" class="border-bottom">
    <b-navbar-brand href="/">Syncabull</b-navbar-brand>

    <b-navbar-toggle target="nav-collapse"></b-navbar-toggle>

    <b-collapse id="nav-collapse" is-nav>
      <b-navbar-nav>
        <b-nav-item href="about" disabled>About</b-nav-item>
        <b-nav-item href="#" disabled>Repository</b-nav-item>
      </b-navbar-nav>

      <!-- Right aligned nav items -->
      <!-- <b-navbar-nav class="ml-auto"> -->
        <!-- <b-nav-item href="/login">Login</b-nav-item> -->

      <!-- </b-navbar-nav> -->
    </b-collapse>

    <!-- TODO: UPDATE this to respect logged in vs not -->
    <b-navbar-nav class="ml-auto">
      <b-button size="lg" variant="primary" class="font-weight-bold" href="/login">Start Syncing</b-button>
    </b-navbar-nav>
  </b-navbar>
</div>
</template>

<script>
export default {
  name: 'Nav',
}
</script>
