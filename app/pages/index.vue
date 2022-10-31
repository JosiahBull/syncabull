<template>
  <div>
    <Head></Head>
    <Nav id="nav"></Nav>
    <b-container id="hero" class="slide-up-fade-in">
      <b-row>
        <h1 class="mx-auto align-middle text-center font-weight-bolder sans">Backup Your <br>Google Photos</h1>
      </b-row>
      <b-row>
        <LoginButton class="mx-auto align-middle font-weight-bold"></LoginButton>
      </b-row>
    </b-container>
    <Footer></Footer>
  </div>
</template>

<script lang="ts">
import Vue from 'vue'
import LoginButton from '~/components/LoginButton.vue';

function vert_cent_hero(element: any) {
  const window_height = window.innerHeight;
  const hero: any = document.querySelector('#hero');
  element.style.marginTop = (window_height / 2) - element.offsetHeight + 'px';
}

export default Vue.extend({
    name: "IndexPage",
    beforeMount() {
        const el = document.querySelector("#hero");
        vert_cent_hero(el);
        window.addEventListener("resize", vert_cent_hero.bind(null, el));
    },
    components: { LoginButton }
})

</script>

<style>
#hero {
  margin-top: calc(50vh - 150px);
  opacity: 0;
}

.outer-square {
  width: 350px;
  height: 350px;
  background: #bbb;
  opacity: 0.5;
  margin: 0 auto;
  padding: 50px;
  margin-top: 50px;
}

.square {
  width: 310px;
  height: 320px;
  background: #ddd;
  margin: 0 auto;
  position: relative;
  opacity: 0.9;
  padding: 15px 20px;
  border: 1px solid #eee;
}

h3 {
  text-align: center;
}

p {
  margin-bottom: 15px;
}

.slide-up-fade-in {
  animation: slide-up-fade-in ease 1s;
  animation-iteration-count: 1;
  transform-origin: 50% 50%;
  animation-fill-mode: forwards;
  /*when the spec is finished*/
  -webkit-animation: slide-up-fade-in ease 1s;
  -webkit-animation-iteration-count: 1;
  -webkit-transform-origin: 50% 50%;
  -webkit-animation-fill-mode: forwards;
  /*Chrome 16+, Safari 4+*/
  -moz-animation: slide-up-fade-in ease 1s;
  -moz-animation-iteration-count: 1;
  -moz-transform-origin: 50% 50%;
  -moz-animation-fill-mode: forwards;
  /*FF 5+*/
  -o-animation: slide-up-fade-in ease 1s;
  -o-animation-iteration-count: 1;
  -o-transform-origin: 50% 50%;
  -o-animation-fill-mode: forwards;
  /*Not implemented yet*/
  -ms-animation: slide-up-fade-in ease 1s;
  -ms-animation-iteration-count: 1;
  -ms-transform-origin: 50% 50%;
  -ms-animation-fill-mode: forwards;
  /*IE 10+*/

  opacity: 0;
  opacity: 1\9;

}


@keyframes slide-up-fade-in {
  0% {
    opacity: 0;
    transform: translate(0px, 40px);
  }

  100% {
    opacity: 1;
    transform: translate(0px, 0px);
  }
}

@-moz-keyframes slide-up-fade-in {
  0% {
    opacity: 0;
    -moz-transform: translate(0px, 40px);
  }

  100% {
    opacity: 1;
    -moz-transform: translate(0px, 0px);
  }
}

@-webkit-keyframes slide-up-fade-in {
  0% {
    opacity: 0;
    -webkit-transform: translate(0px, 40px);
  }

  100% {
    opacity: 1;
    -webkit-transform: translate(0px, 0px);
  }
}

@-o-keyframes slide-up-fade-in {
  0% {
    opacity: 0;
    -o-transform: translate(0px, 40px);
  }

  100% {
    opacity: 1;
    -o-transform: translate(0px, 0px);
  }
}

@-ms-keyframes slide-up-fade-in {
  0% {
    opacity: 0;
    -ms-transform: translate(0px, 40px);
  }

  100% {
    opacity: 1;
    -ms-transform: translate(0px, 0px);
  }
}


.clearfix {
  clear: both;
  zoom: 1;
}
</style>
