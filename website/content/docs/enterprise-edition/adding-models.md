+++
title = "Adding LLM Models"
weight = 95
sort_by = "weight"
+++

To add a model to your cluster you can create a [Deployment](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/) and a [Service](https://kubernetes.io/docs/concepts/services-networking/service/).

## Example Adding Mixtral 8x7B (Work in progress)

This section is a work in progress but deploying a new model will look something like this.

```yml
apiVersion: v1
kind: Service
metadata:
  name: mixtral-8x7b
  namespace: bionic-gpt
spec:
  selector:
    app: mixtral-8x7b
  ports:
    - protocol: TCP
      port: 8000
      targetPort: 8000

---

apiVersion: apps/v1
kind: Deployment
metadata:
  name: mixtral-8x7b-deployment
  namespace: bionic-gpt
spec:
  replicas: 1
  selector:
    matchLabels:
      app: mixtral-8x7b
  template:
    metadata:
      labels:
        app: mixtral-8x7b
    spec:
      containers:
      - name: bionic-gpt-operator
        image: ghcr.io/huggingface/text-generation-inference:sha-0eabc83
        args: 
          - --model-id 
          - mistralai/Mixtral-8x7B-Instruct-v0.1
          - --quantize 
          - gptq
```

The model and inference engine must run in the same container.