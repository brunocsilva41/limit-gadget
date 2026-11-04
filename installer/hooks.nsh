; Hooks do instalador NSIS do LimitGadget (ver installer/README.md).
; Este arquivo é incluído pelo modelo do Tauri antes da definição das páginas,
; por isso os !define abaixo valem para a página de boas-vindas e para a de conclusão.
; Em modo silencioso (/S) e passivo (/P, usado pelo atualizador) nenhuma página aparece
; e nenhuma das escolhas abaixo é alterada.

; ---- Textos das páginas ----
!define MUI_WELCOMEPAGE_TITLE "Instalar o LimitGadget"
!define MUI_WELCOMEPAGE_TEXT "Acompanhe os limites de uso das suas IAs numa barra discreta sobre a barra de tarefas.$\r$\n$\r$\nA instalação é só para o seu usuário e não pede administrador."
!define MUI_FINISHPAGE_TITLE "Tudo pronto"
!define MUI_FINISHPAGE_TEXT "O LimitGadget foi instalado."
!define MUI_FINISHPAGE_RUN_TEXT "Abrir o LimitGadget"
; Atalho na área de trabalho começa desmarcado
!define MUI_FINISHPAGE_SHOWREADME_NOTCHECKED

; ---- Opção "Iniciar com o Windows" na página de boas-vindas ----
!define MUI_PAGE_CUSTOMFUNCTION_SHOW LGWelcomeShow
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE LGWelcomeLeave

Var LGAutostartCheckbox
Var LGAutostartState

Function LGWelcomeShow
  ; O diálogo de boas-vindas (nsDialogs) já existe quando esta função roda
  ${NSD_CreateCheckBox} 120u 176u 195u 12u "Iniciar com o Windows"
  Pop $LGAutostartCheckbox
  SetCtlColors $LGAutostartCheckbox "000000" "FFFFFF"

  ; Marca a opção se o início com o Windows já estiver ativo
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  ${If} $0 != ""
    ${NSD_Check} $LGAutostartCheckbox
  ${EndIf}
FunctionEnd

Function LGWelcomeLeave
  ${NSD_GetState} $LGAutostartCheckbox $LGAutostartState
FunctionEnd

; ---- Instalação ----
!macro NSIS_HOOK_POSTINSTALL
  ; $LGAutostartState só é 1 quando a pessoa marcou a opção na página de boas-vindas.
  ; Em modo silencioso, passivo ou atualização a página não aparece e o valor Run não é alterado.
  ${If} $LGAutostartState = 1
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\""
  ${EndIf}
!macroend

; ---- Desinstalação ----
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    ; Remove o início com o Windows e a lembrança da pasta de instalação
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
    DeleteRegKey HKCU "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty HKCU "${MANUKEY}"

    ; Dados do app (%APPDATA%\LimitGadget): só apaga se a pessoa marcou a caixa da tela
    ; de desinstalação. Em modo silencioso ou passivo a tela não aparece e nada é apagado.
    ${If} $DeleteAppDataCheckboxState = 1
    ${AndIf} $PassiveMode <> 1
      SetShellVarContext current
      RmDir /r "$APPDATA\LimitGadget"
    ${EndIf}
  ${EndIf}
!macroend
