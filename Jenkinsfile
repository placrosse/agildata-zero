 node ('linux'){
  stage 'Build and Test'
  checkout scm
  sh 'cargo test'
  sh 'scripts/jenkins/test.sh'
 }
