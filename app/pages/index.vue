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

    <FloatingBoxes></FloatingBoxes>

    <!-- Information about the application -->
    <!-- There is an arrow with "more info" at the bottom of the landing page  -->
    <!-- 3x2 grid of cards, each containing an image, title, and short description -->

    <b-container id="info">

      <b-col class="mx-auto align-middle text-center clickable" @click="onClickScroll('#info')">
        <p class="mx-auto align-middle text-center text-muted m-0 p-0">More</p>
        <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" fill="currentColor" class="text-muted mb-5"
          viewBox="0 0 16 16">
          <path fill-rule="evenodd"
            d="M1.646 4.646a.5.5 0 0 1 .708 0L8 10.293l5.646-5.647a.5.5 0 0 1 .708.708l-6 6a.5.5 0 0 1-.708 0l-6-6a.5.5 0 0 1 0-.708z" />
        </svg>
      </b-col>
      <!-- Title -->
      <b-row>
        <h2 class="mx-auto align-middle text-center light">Why Backup Your Photos?</h2>
      </b-row>

      <!-- 3x2 grid of cards, each containing an image, title, and short description -->
      <span>
        <b-row>
          <b-col cols="12" md="4" class="mb-4">
            <b-card class="h-100">
              <b-card-img src="https://picsum.photos/300/200?image=0" alt="Image 0"></b-card-img>
              <b-card-body>
                <b-card-title>Open Source</b-card-title>
                <b-card-text>
                  This application is open source, and always will be.
                </b-card-text>
              </b-card-body>
            </b-card>
          </b-col>

          <b-col cols="12" md="4" class="mb-4">
            <b-card class="h-100">
              <b-card-img src="https://picsum.photos/300/200?image=1" alt="Image 1"></b-card-img>
              <b-card-body>
                <b-card-title>Secure, and Encrypted</b-card-title>
                <b-card-text>
                  Your files are encrypted while in transit, and aren't stored on our servers.
                </b-card-text>
              </b-card-body>
            </b-card>
          </b-col>

          <b-col cols="12" md="4" class="mb-4">
            <b-card class="h-100">
              <b-card-img src="https://picsum.photos/300/200?image=2" alt="Image 2"></b-card-img>
              <b-card-body>
                <b-card-title>Self Hostable</b-card-title>
                <b-card-text>
                  You can host this application yourself, and have full control over your data.
                </b-card-text>
              </b-card-body>
            </b-card>
          </b-col>
        </b-row>

        <b-row>
          <b-col cols="12" md="4" class="mb-4">
            <b-card class="h-100">
              <b-card-img src="https://picsum.photos/300/200?image=0" alt="Image 0"></b-card-img>
              <b-card-body>
                <b-card-title>Powered by Rust and Docker</b-card-title>
                <b-card-text>
                  This application is built with Rust, and Docker. Blazing fast, secure, and reliable.
                </b-card-text>
              </b-card-body>
            </b-card>
          </b-col>

          <b-col cols="12" md="4" class="mb-4">
            <b-card class="h-100">
              <b-card-img src="https://picsum.photos/300/200?image=1" alt="Image 1"></b-card-img>
              <b-card-body>
                <b-card-title>Fast and Stable</b-card-title>
                <b-card-text>
                  We have committed to a thorough testing process to ensure high levels of uptime. Your backups will always be safe.
                </b-card-text>
              </b-card-body>
            </b-card>
          </b-col>

          <b-col cols="12" md="4" class="mb-4">
            <b-card class="h-100">
              <b-card-img src="https://picsum.photos/300/200?image=2" alt="Image 2"></b-card-img>
              <b-card-body>
                <b-card-title>Filter Duplicates</b-card-title>
                <b-card-text>
                  We won't store duplicate images in your backup. We'll only store the highest quality version of each image.
                </b-card-text>
              </b-card-body>
            </b-card>
          </b-col>
        </b-row>
      </span>

    </b-container>

    <Footer></Footer>
  </div>
</template>

<script>
import FloatingBoxes from '~/components/FloatingBoxes.vue';
import Vue from 'vue'
import LoginButton from '~/components/LoginButton.vue';

function vertCentHero(element) {
  const window_height = window.innerHeight;
  const hero = document.querySelector('#hero');
  element.style.marginTop = (window_height / 2) - element.offsetHeight + 'px';
}

export default Vue.extend({
  name: "IndexPage",
  beforeMount() {
    const el = document.querySelector("#hero");
    vertCentHero(el);
    window.addEventListener("resize", vertCentHero.bind(null, el));
  },
  components: { LoginButton, FloatingBoxes },
  methods: {
    onClickScroll: function (element) {
      const el = document.querySelector(element);
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }
})

</script>

<style>
#hero {
  margin-top: calc(50vh - 150px);
  opacity: 0;
  /* upwards box shadow to differentiate */
}

#hero h1 {
  font-size: 3rem;
}

#info {
  /* start 50vh + 150px */
  margin-top: calc(40vh);
}

.clickable {
  cursor: pointer;
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
