.PHONY: setup doctor info run run-release sync test lint fmt upgrade clean check bootstrap aws-check

setup:
	mise run setup

doctor:
	mise run doctor

info:
	mise run info

run:
	mise run run

run-release:
	mise run run-release

sync:
	mise run sync

test:
	mise run test

lint:
	mise run lint

fmt:
	mise run fmt

upgrade:
	mise run upgrade

clean:
	mise run clean

check:
	mise run check

bootstrap:
	mise run bootstrap

aws-check:
	mise run aws-check
