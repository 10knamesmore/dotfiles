function on-chpwd() {
  if [[ $PWD = $HOME ]]; then
    clear
  fi
}

autoload -Uz add-zsh-hook
add-zsh-hook chpwd on-chpwd 
