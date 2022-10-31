<template>
<div>
    <b-button v-if="!logged_in" size="lg" variant="primary" class="font-weight-bold" v-bind:href="login_link">Start Syncing</b-button>
    <b-button v-if="logged_in" size="lg" variant="primary" class="font-weight-bold" href="/dashboard">Dashboard</b-button>
</div>
</template>

<script>
export default {
  name: 'LoginButton',
  data: function() {
    return {
      logged_in: false,
      login_link: '/api/1/login'
    }
  },
  beforeMount() {
    let user = localStorage.getItem('user');
    if (user) {
      this.logged_in = true;
    }
  }
}
</script>
