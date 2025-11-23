
<img src="https://github.com/Kerimniy/ServerStatMonitoring/blob/main/app_win_x86_64/static/favicon.png" style="width: 70px">

<p>In english <a href="https://github.com/Kerimniy/ServerStatMonitoring/blob/main/README.md">click</a></p>

# ServerStatMonitoring — веб‑приложение для мониторинга в реальном времени

Простая одностраничная панель мониторинга для отслеживания состояния компьютера (ЦП, оперативной памяти, дисков, операционной системы).  
Работает на **Windows** и **Linux**.

<p align="center">
<img src="https://github.com/Kerimniy/ServerStatMonitoring/blob/main/prev.png" alt="logo" width="70%">
</p>

## Возможности

- Веб‑интерфейс, построенный с использованием шаблонов **Rocket.rs**  
- Обновление данных каждые **10 секунд** без перезагрузки страницы  
- Графики загрузки ЦП и оперативной памяти за последнюю минуту  
- Поддержка Windows и Linux (автоматическое определение имени ОС)  
- Не требует внешнего сервера — работает как один исполняемый файл  

## Используемые библиотеки

- **Rocket** — веб‑фреймворк  
- **sysinfo** — сбор системной информации  
- **tokio** — асинхронное обновление данных в фоновом режиме  
- **serde + rocket_dyn_templates** — JSON‑API и шаблонизация  
- **chrono** — работа с временем  

***
